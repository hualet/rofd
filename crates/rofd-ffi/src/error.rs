use crate::abi::ROFD_RENDER_DIAGNOSTIC_V1_SIZE;
use crate::handles::{drop_raw_handle, handle_ref, into_raw_handle, ErrorHandle, HandleToken};
use crate::{
    rofd_error_t, rofd_render_diagnostic_t, rofd_status_t, ROFD_STATUS_INTERNAL,
    ROFD_STATUS_INVALID_ARGUMENT, ROFD_STATUS_INVALID_DOCUMENT, ROFD_STATUS_IO,
    ROFD_STATUS_LIMIT_EXCEEDED, ROFD_STATUS_OK, ROFD_STATUS_OUT_OF_MEMORY,
    ROFD_STATUS_PAGE_OUT_OF_RANGE, ROFD_STATUS_RENDER_ERROR, ROFD_STATUS_UNSUPPORTED,
};
use std::any::Any;
use std::ffi::{c_char, CString};
use std::mem::{align_of, size_of};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::ptr;

#[allow(dead_code)] // Consumed by document and renderer entry points added in subsequent tasks.
pub(crate) struct FfiError {
    status: rofd_status_t,
    message: CString,
}

#[allow(dead_code)] // Consumed by document and renderer entry points added in subsequent tasks.
impl FfiError {
    pub(crate) fn new(status: rofd_status_t, message: impl AsRef<str>) -> Self {
        let visible_message = message.as_ref().replace('\0', "\\0");
        Self {
            status,
            message: CString::new(visible_message)
                .expect("replacing embedded NUL bytes must produce a valid C string"),
        }
    }

    pub(crate) fn invalid_argument(message: impl AsRef<str>) -> Self {
        Self::new(ROFD_STATUS_INVALID_ARGUMENT, message)
    }

    fn status(&self) -> rofd_status_t {
        self.status
    }

    #[cfg(test)]
    fn message(&self) -> &CString {
        &self.message
    }

    fn into_raw(self) -> *mut rofd_error_t {
        into_raw_handle::<rofd_error_t>(Box::new(ErrorHandle {
            status: self.status,
            message: self.message,
        }))
    }
}

#[allow(dead_code)] // Consumed by document entry points added in the next task.
impl From<rofd_core::Error> for FfiError {
    fn from(error: rofd_core::Error) -> Self {
        Self::new(core_status(&error), error.to_string())
    }
}

#[allow(dead_code)] // Consumed by renderer entry points added in a subsequent task.
impl From<rofd_render::Error> for FfiError {
    fn from(error: rofd_render::Error) -> Self {
        Self::new(render_status(&error), error.to_string())
    }
}

#[allow(dead_code)] // Consumed by document entry points added in the next task.
pub(crate) fn core_status(error: &rofd_core::Error) -> rofd_status_t {
    match error {
        rofd_core::Error::Internal(_) => ROFD_STATUS_INTERNAL,
        rofd_core::Error::Io { .. } => ROFD_STATUS_IO,
        rofd_core::Error::LimitExceeded(_) => ROFD_STATUS_LIMIT_EXCEEDED,
        rofd_core::Error::UnsupportedFeature(_) => ROFD_STATUS_UNSUPPORTED,
        rofd_core::Error::PageOutOfRange { .. } => ROFD_STATUS_PAGE_OUT_OF_RANGE,
        rofd_core::Error::Container(_)
        | rofd_core::Error::Xml { .. }
        | rofd_core::Error::InvalidStructure { .. }
        | rofd_core::Error::InvalidValue { .. }
        | rofd_core::Error::MissingEntry(_)
        | rofd_core::Error::UnknownResource { .. }
        | rofd_core::Error::ResourceKindMismatch { .. }
        | rofd_core::Error::DuplicateResourceId { .. }
        | rofd_core::Error::InvalidResource { .. }
        | rofd_core::Error::InvalidPageObject { .. } => ROFD_STATUS_INVALID_DOCUMENT,
        _ => ROFD_STATUS_INTERNAL,
    }
}

#[allow(dead_code)] // Consumed by renderer entry points added in a subsequent task.
pub(crate) fn render_status(error: &rofd_render::Error) -> rofd_status_t {
    match error {
        rofd_render::Error::InvalidOption { .. } => ROFD_STATUS_INVALID_ARGUMENT,
        rofd_render::Error::RasterBudgetExceeded { .. }
        | rofd_render::Error::FontBytesExceeded { .. }
        | rofd_render::Error::DisplayListImageBudgetExceeded { .. }
        | rofd_render::Error::ImageLimitExceeded { .. } => ROFD_STATUS_LIMIT_EXCEEDED,
        rofd_render::Error::RasterAllocation { .. } => ROFD_STATUS_OUT_OF_MEMORY,
        rofd_render::Error::UnsupportedDisplayCommand { .. }
        | rofd_render::Error::UnsupportedImageFormat { .. } => ROFD_STATUS_UNSUPPORTED,
        rofd_render::Error::ObjectResource { source, .. } => core_status(source),
        rofd_render::Error::ObjectResourceProcessing { source, .. } => render_status(source),
        rofd_render::Error::FontCache { .. } | rofd_render::Error::ImageCache { .. } => {
            ROFD_STATUS_INTERNAL
        }
        rofd_render::Error::InvalidSurfaceSize { .. }
        | rofd_render::Error::SurfaceTooSmall { .. }
        | rofd_render::Error::InvalidModel { .. }
        | rofd_render::Error::InvalidDisplayList { .. }
        | rofd_render::Error::InvalidGeometry { .. }
        | rofd_render::Error::InvalidImageSurfaceSize { .. }
        | rofd_render::Error::Backend { .. }
        | rofd_render::Error::Cleanup { .. }
        | rofd_render::Error::InvalidFont { .. }
        | rofd_render::Error::InvalidGlyph { .. }
        | rofd_render::Error::InvalidTextLayout { .. }
        | rofd_render::Error::FontBackend { .. }
        | rofd_render::Error::FontResourceMismatch { .. }
        | rofd_render::Error::ImageFormatMismatch { .. }
        | rofd_render::Error::ImageDecode { .. }
        | rofd_render::Error::InvalidImageDimensions { .. }
        | rofd_render::Error::ImageDimensionsOverflow { .. } => ROFD_STATUS_RENDER_ERROR,
        _ => ROFD_STATUS_INTERNAL,
    }
}

#[allow(dead_code)] // Consumed by fallible entry points added in subsequent tasks.
struct ErrorSlot {
    output: *mut *mut rofd_error_t,
}

#[allow(dead_code)] // Consumed by fallible entry points added in subsequent tasks.
impl ErrorSlot {
    unsafe fn new(output: *mut *mut rofd_error_t) -> Self {
        if !output.is_null() {
            // SAFETY: boundary validated this non-null error output as aligned,
            // non-overflowing, writable under the caller contract, and disjoint
            // from ordinary outputs.
            unsafe { output.write(ptr::null_mut()) };
        }
        Self { output }
    }

    unsafe fn publish(&mut self, error: FfiError) -> rofd_status_t {
        let status = error.status();
        if !self.output.is_null() {
            let raw = error.into_raw();
            // SAFETY: ErrorSlot construction established the same caller-provided writable slot,
            // and this is its only publication write during the boundary call.
            unsafe { self.output.write(raw) };
        }
        status
    }
}

#[allow(dead_code)] // Consumed by fallible entry points added in subsequent tasks.
fn panic_error(payload: Box<dyn Any + Send>) -> FfiError {
    let message = if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_owned()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "non-string panic payload".to_owned()
    };
    FfiError::new(
        ROFD_STATUS_INTERNAL,
        format!("panic at FFI boundary: {message}"),
    )
}

#[allow(dead_code)] // Consumed by fallible entry points added in subsequent tasks.
pub(crate) struct HandleOutput<Token: HandleToken> {
    output: *mut *mut Token,
}

#[allow(dead_code)] // Consumed by fallible entry points added in subsequent tasks.
impl<Token: HandleToken> HandleOutput<Token> {
    pub(crate) fn required(output: *mut *mut Token) -> Self {
        Self { output }
    }
}

#[allow(dead_code)] // Consumed by fallible entry points added in subsequent tasks.
pub(crate) struct ScalarOutput<T> {
    output: *mut T,
}

pub(crate) struct DiagnosticFields {
    pub(crate) kind: u32,
    pub(crate) object_id: u64,
    pub(crate) message: *const c_char,
}

pub(crate) struct DiagnosticOutput {
    output: *mut rofd_render_diagnostic_t,
    declared_size: u32,
}

impl DiagnosticOutput {
    /// Captures the caller's input `struct_size` before the output transaction.
    ///
    /// # Safety
    ///
    /// A non-null pointer must be readable for its first `u32`; alignment is
    /// checked by the transaction before any output write.
    pub(crate) unsafe fn required(output: *mut rofd_render_diagnostic_t) -> Self {
        let declared_size = if output.is_null() {
            0
        } else {
            // SAFETY: The caller provides an initialized, readable prefix field.
            unsafe { output.cast::<u32>().read_unaligned() }
        };
        Self {
            output,
            declared_size,
        }
    }
}

#[allow(dead_code)] // Consumed by fallible entry points added in subsequent tasks.
impl<T> ScalarOutput<T> {
    pub(crate) fn required(output: *mut T) -> Self {
        Self { output }
    }
}

/// C scalar types whose entry default is a compile-time zero value.
///
/// Keeping this trait private to the crate prevents output initialization from
/// dispatching arbitrary `Default` code that could panic before writing a slot.
#[allow(private_bounds)]
pub(crate) trait ZeroScalar: scalar_private::Sealed + Copy {
    const ZERO: Self;
}

mod scalar_private {
    pub(super) trait Sealed {}
}

macro_rules! zero_scalars {
    ($($type:ty),+ $(,)?) => {
        $(
            impl scalar_private::Sealed for $type {}

            impl ZeroScalar for $type {
                const ZERO: Self = 0 as Self;
            }
        )+
    };
}

zero_scalars!(i32, u32, u64, usize, f64);

impl scalar_private::Sealed for crate::rofd_rect_t {}

impl ZeroScalar for crate::rofd_rect_t {
    const ZERO: Self = Self {
        x_mm: 0.0,
        y_mm: 0.0,
        width_mm: 0.0,
        height_mm: 0.0,
    };
}

#[derive(Clone, Copy, Default)]
struct SlotRange {
    start: usize,
    end: usize,
}

impl SlotRange {
    fn for_pointer<T>(pointer: *mut T) -> Result<Option<Self>, ()> {
        if pointer.is_null() {
            return Ok(None);
        }
        let start = pointer as usize;
        if !start.is_multiple_of(align_of::<T>()) {
            return Err(());
        }
        let end = start.checked_add(size_of::<T>()).ok_or(())?;
        Ok(Some(Self { start, end }))
    }

    fn overlaps(self, other: Self) -> bool {
        self.start < other.end && other.start < self.end
    }

    fn for_region(pointer: *const u8, size: usize, alignment: usize) -> Result<Option<Self>, ()> {
        if pointer.is_null() {
            return Ok(None);
        }
        let start = pointer as usize;
        if !start.is_multiple_of(alignment) {
            return Err(());
        }
        let end = start.checked_add(size).ok_or(())?;
        Ok(Some(Self { start, end }))
    }
}

const MAX_OUTPUT_SLOTS: usize = 3;

#[derive(Default)]
pub(crate) struct SlotRanges {
    slots: [SlotRange; MAX_OUTPUT_SLOTS],
    len: usize,
}

impl SlotRanges {
    fn push<T>(&mut self, pointer: *mut T) -> Result<(), ()> {
        let Some(range) = SlotRange::for_pointer(pointer)? else {
            return Ok(());
        };
        if self.len == self.slots.len() {
            return Err(());
        }
        self.slots[self.len] = range;
        self.len += 1;
        Ok(())
    }

    fn iter(&self) -> impl Iterator<Item = SlotRange> + '_ {
        self.slots[..self.len].iter().copied()
    }

    fn has_overlap(&self) -> bool {
        (0..self.len).any(|left| {
            ((left + 1)..self.len).any(|right| self.slots[left].overlaps(self.slots[right]))
        })
    }
}

const MAX_INPUT_REGIONS: usize = 5;

/// Read-only regions that must remain disjoint from every FFI output slot.
#[derive(Default)]
pub(crate) struct InputRanges {
    regions: [SlotRange; MAX_INPUT_REGIONS],
    len: usize,
}

impl InputRanges {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn push<T>(&mut self, pointer: *const T) -> Result<(), ()> {
        self.push_region(pointer.cast(), size_of::<T>(), align_of::<T>())
    }

    pub(crate) fn push_region(
        &mut self,
        pointer: *const u8,
        size: usize,
        alignment: usize,
    ) -> Result<(), ()> {
        let range = SlotRange::for_region(pointer, size, alignment)?;
        let Some(range) = range else {
            return Ok(());
        };
        if self.len == self.regions.len() {
            return Err(());
        }
        self.regions[self.len] = range;
        self.len += 1;
        Ok(())
    }

    fn iter(&self) -> impl Iterator<Item = SlotRange> + '_ {
        self.regions[..self.len].iter().copied()
    }
}

mod output_private {
    pub(super) trait Slot {}
    pub(super) trait Set {}
}

/// One caller output whose entry initialization and success publication consist
/// only of non-panicking raw writes.
///
/// # Safety
///
/// Implementations must initialize every valid non-null slot even when another
/// slot returns `false`. A versioned record may leave itself untouched when its
/// declared prefix is too small to initialize safely. `commit` may only perform
/// operations that cannot unwind, and must consume or publish every staged value
/// exactly once.
#[allow(private_bounds)]
pub(crate) unsafe trait OutputSlot: output_private::Slot {
    type Staged;

    fn collect_range(&self, ranges: &mut SlotRanges) -> Result<(), ()>;
    unsafe fn initialize(&self) -> bool;
    unsafe fn commit(self, staged: Self::Staged);
}

impl<Token: HandleToken> output_private::Slot for HandleOutput<Token> {}

unsafe impl<Token: HandleToken> OutputSlot for HandleOutput<Token> {
    type Staged = Box<Token::Storage>;

    fn collect_range(&self, ranges: &mut SlotRanges) -> Result<(), ()> {
        ranges.push(self.output)
    }

    unsafe fn initialize(&self) -> bool {
        if self.output.is_null() {
            return false;
        }
        // SAFETY: A non-null required output is caller-provided writable storage for one token
        // pointer. No Storage ownership exists yet, so NULL is the only entry value.
        unsafe { self.output.write(ptr::null_mut()) };
        true
    }

    unsafe fn commit(self, staged: Self::Staged) {
        // SAFETY: boundary calls commit only after initialize returned true. Box::into_raw cannot
        // unwind and transfers the allocation exactly once; pointer publication is a raw write
        // with no user code or destructor invocation.
        unsafe { self.output.write(into_raw_handle::<Token>(staged)) };
    }
}

impl<T: ZeroScalar> output_private::Slot for ScalarOutput<T> {}

unsafe impl<T: ZeroScalar> OutputSlot for ScalarOutput<T> {
    type Staged = T;

    fn collect_range(&self, ranges: &mut SlotRanges) -> Result<(), ()> {
        ranges.push(self.output)
    }

    unsafe fn initialize(&self) -> bool {
        if self.output.is_null() {
            return false;
        }
        // SAFETY: A non-null required output is caller-provided writable storage for one T.
        // T is one of this module's sealed-by-construction scalar implementations, so obtaining
        // its constant zero value cannot invoke user code or panic.
        unsafe { self.output.write(T::ZERO) };
        true
    }

    unsafe fn commit(self, staged: Self::Staged) {
        // SAFETY: boundary calls commit only after initialize returned true. A raw write of Copy
        // data invokes no destructor and cannot unwind.
        unsafe { self.output.write(staged) };
    }
}

impl output_private::Slot for DiagnosticOutput {}

unsafe impl OutputSlot for DiagnosticOutput {
    type Staged = DiagnosticFields;

    fn collect_range(&self, ranges: &mut SlotRanges) -> Result<(), ()> {
        if self.declared_size as usize >= ROFD_RENDER_DIAGNOSTIC_V1_SIZE {
            ranges.push(self.output)
        } else {
            ranges.push(self.output.cast::<u32>())
        }
    }

    unsafe fn initialize(&self) -> bool {
        if self.output.is_null() || (self.declared_size as usize) < ROFD_RENDER_DIAGNOSTIC_V1_SIZE {
            return false;
        }
        // SAFETY: The declared supported boundary guarantees a writable complete v1 prefix.
        // Byte clearing includes all internal and tail padding before individual fields are set.
        unsafe {
            ptr::write_bytes(self.output.cast::<u8>(), 0, ROFD_RENDER_DIAGNOSTIC_V1_SIZE);
            ptr::addr_of_mut!((*self.output).struct_size).write(self.declared_size);
        }
        true
    }

    unsafe fn commit(self, staged: Self::Staged) {
        // SAFETY: initialize validated and cleared the complete v1 record; raw field writes are
        // non-panicking and keep the caller's captured struct_size and unknown tail unchanged.
        unsafe {
            ptr::addr_of_mut!((*self.output).kind).write(staged.kind);
            ptr::addr_of_mut!((*self.output).object_id).write(staged.object_id);
            ptr::addr_of_mut!((*self.output).message).write(staged.message);
        }
    }
}

/// A complete transaction over all outputs of one FFI operation.
///
/// # Safety
///
/// Implementations must visit every slot during `initialize`, without
/// short-circuiting, and their `commit` implementation must not unwind.
#[allow(private_bounds)]
pub(crate) unsafe trait OutputSet: output_private::Set {
    type Staged;

    fn ranges(&self) -> Result<SlotRanges, ()>;
    unsafe fn initialize(&self) -> bool;
    unsafe fn commit(self, staged: Self::Staged);
}

impl output_private::Set for () {}

unsafe impl OutputSet for () {
    type Staged = ();

    fn ranges(&self) -> Result<SlotRanges, ()> {
        Ok(SlotRanges::default())
    }

    unsafe fn initialize(&self) -> bool {
        true
    }

    unsafe fn commit(self, (): Self::Staged) {}
}

impl<Token: HandleToken> output_private::Set for HandleOutput<Token> {}

unsafe impl<Token: HandleToken> OutputSet for HandleOutput<Token> {
    type Staged = <Self as OutputSlot>::Staged;

    fn ranges(&self) -> Result<SlotRanges, ()> {
        let mut ranges = SlotRanges::default();
        self.collect_range(&mut ranges)?;
        Ok(ranges)
    }

    unsafe fn initialize(&self) -> bool {
        // SAFETY: Delegate to this set's sole slot while preserving its contract.
        unsafe { OutputSlot::initialize(self) }
    }

    unsafe fn commit(self, staged: Self::Staged) {
        // SAFETY: Delegate to this set's sole slot after successful initialization.
        unsafe { OutputSlot::commit(self, staged) };
    }
}

impl<T: ZeroScalar> output_private::Set for ScalarOutput<T> {}

unsafe impl<T: ZeroScalar> OutputSet for ScalarOutput<T> {
    type Staged = <Self as OutputSlot>::Staged;

    fn ranges(&self) -> Result<SlotRanges, ()> {
        let mut ranges = SlotRanges::default();
        self.collect_range(&mut ranges)?;
        Ok(ranges)
    }

    unsafe fn initialize(&self) -> bool {
        // SAFETY: Delegate to this set's sole slot while preserving its contract.
        unsafe { OutputSlot::initialize(self) }
    }

    unsafe fn commit(self, staged: Self::Staged) {
        // SAFETY: Delegate to this set's sole slot after successful initialization.
        unsafe { OutputSlot::commit(self, staged) };
    }
}

impl output_private::Set for DiagnosticOutput {}

unsafe impl OutputSet for DiagnosticOutput {
    type Staged = <Self as OutputSlot>::Staged;

    fn ranges(&self) -> Result<SlotRanges, ()> {
        let mut ranges = SlotRanges::default();
        self.collect_range(&mut ranges)?;
        Ok(ranges)
    }

    unsafe fn initialize(&self) -> bool {
        // SAFETY: Delegate to this set's sole record slot while preserving its contract.
        unsafe { OutputSlot::initialize(self) }
    }

    unsafe fn commit(self, staged: Self::Staged) {
        // SAFETY: Delegate after successful transactional initialization.
        unsafe { OutputSlot::commit(self, staged) };
    }
}

macro_rules! output_set_tuple {
    ($(($slot_type:ident, $slot:ident, $staged:ident)),+ $(,)?) => {
        impl<$($slot_type: OutputSlot),+> output_private::Set for ($($slot_type,)+) {}

        unsafe impl<$($slot_type: OutputSlot),+> OutputSet for ($($slot_type,)+) {
            type Staged = ($($slot_type::Staged,)+);

            fn ranges(&self) -> Result<SlotRanges, ()> {
                let ($($slot,)+) = self;
                let mut ranges = SlotRanges::default();
                $($slot.collect_range(&mut ranges)?;)+
                Ok(ranges)
            }

            unsafe fn initialize(&self) -> bool {
                let ($($slot,)+) = self;
                let mut all_valid = true;
                $(
                    // Bitwise assignment intentionally avoids boolean short-circuiting: every
                    // non-null output must receive its entry default even if another is invalid.
                    // SAFETY: Each tuple member is initialized exactly once.
                    all_valid &= unsafe { $slot.initialize() };
                )+
                all_valid
            }

            unsafe fn commit(self, staged: Self::Staged) {
                let ($($slot,)+) = self;
                let ($($staged,)+) = staged;
                $(
                    // SAFETY: boundary commits only after every slot initialized successfully;
                    // OutputSlot requires this ownership transfer to be non-panicking.
                    unsafe { $slot.commit($staged) };
                )+
            }
        }
    };
}

output_set_tuple!((A, a, staged_a), (B, b, staged_b));
output_set_tuple!((A, a, staged_a), (B, b, staged_b), (C, c, staged_c));

#[allow(dead_code)] // Consumed by fallible entry points added in subsequent tasks.
/// Runs one fallible FFI operation with validated transactional outputs.
///
/// Before any raw write, the boundary rejects output address overflow,
/// misalignment, and overlap. It then initializes every non-null ordinary
/// output, runs `operation`, and publishes its staged values only on success.
///
/// # Safety
///
/// Every non-null output and error slot must point to writable storage for its
/// declared type. The slots must remain live for the call and must not be
/// accessed concurrently. Detectable overlap is rejected, but validity of an
/// otherwise well-formed process address cannot be proven by the library.
pub(crate) unsafe fn boundary<Outputs, Operation>(
    error: *mut *mut rofd_error_t,
    outputs: Outputs,
    operation: Operation,
) -> rofd_status_t
where
    Outputs: OutputSet,
    Operation: FnOnce() -> Result<Outputs::Staged, FfiError>,
{
    // SAFETY: This is the no-explicit-input form of the same boundary contract.
    unsafe { boundary_with_inputs(error, outputs, InputRanges::new(), operation) }
}

/// Runs one fallible FFI operation after preflighting input/output disjointness.
///
/// # Safety
///
/// In addition to [`boundary`]'s requirements, every region in `inputs` must
/// describe caller-owned storage that stays readable and immutable for the call.
pub(crate) unsafe fn boundary_with_inputs<Outputs, Operation>(
    error: *mut *mut rofd_error_t,
    outputs: Outputs,
    inputs: InputRanges,
    operation: Operation,
) -> rofd_status_t
where
    Outputs: OutputSet,
    Operation: FnOnce() -> Result<Outputs::Staged, FfiError>,
{
    catch_unwind(AssertUnwindSafe(|| {
        let error_range = match SlotRange::for_pointer(error) {
            Ok(range) => range,
            Err(()) => return ROFD_STATUS_INVALID_ARGUMENT,
        };
        let output_ranges = match outputs.ranges() {
            Ok(ranges) => ranges,
            Err(()) => return ROFD_STATUS_INVALID_ARGUMENT,
        };

        if error_range.is_some_and(|error_range| {
            inputs
                .iter()
                .any(|input_range| error_range.overlaps(input_range))
        }) {
            return ROFD_STATUS_INVALID_ARGUMENT;
        }

        if error_range.is_some_and(|error_range| {
            output_ranges
                .iter()
                .any(|output_range| error_range.overlaps(output_range))
        }) {
            return ROFD_STATUS_INVALID_ARGUMENT;
        }

        if output_ranges.has_overlap() {
            if error.is_null() {
                return ROFD_STATUS_INVALID_ARGUMENT;
            }
            // SAFETY: The error range was validated and proven disjoint from every ordinary
            // output. Ordinary outputs remain untouched on this aliasing failure.
            let mut error_slot = unsafe { ErrorSlot::new(error) };
            // SAFETY: error_slot is the sole validated output written on this failure path.
            return unsafe {
                error_slot.publish(FfiError::invalid_argument("output locations overlap"))
            };
        }

        if output_ranges.iter().any(|output_range| {
            inputs
                .iter()
                .any(|input_range| output_range.overlaps(input_range))
        }) {
            if error.is_null() {
                return ROFD_STATUS_INVALID_ARGUMENT;
            }
            // SAFETY: The error range is validated and disjoint from inputs and ordinary outputs.
            let mut error_slot = unsafe { ErrorSlot::new(error) };
            // SAFETY: error_slot is the sole writable location on this preflight failure.
            return unsafe {
                error_slot.publish(FfiError::invalid_argument(
                    "input and output storage overlap",
                ))
            };
        }

        // SAFETY: The caller of boundary forwards an optional writable error-output slot.
        let mut error_slot = unsafe { ErrorSlot::new(error) };
        // SAFETY: OutputSet guarantees full, non-short-circuiting entry initialization.
        if !unsafe { outputs.initialize() } {
            // SAFETY: error_slot owns the only access to the optional output for this call.
            return unsafe {
                error_slot.publish(FfiError::invalid_argument(
                    "required output is NULL or has an invalid record size",
                ))
            };
        }

        let outcome = catch_unwind(AssertUnwindSafe(operation));
        match outcome {
            Ok(Ok(staged)) => {
                // SAFETY: Every output was validated and initialized. OutputSet's unsafe contract
                // restricts commit to non-panicking raw publication, so partial success cannot be
                // observed and each staged owned value is transferred exactly once.
                unsafe { outputs.commit(staged) };
                ROFD_STATUS_OK
            }
            Ok(Err(error)) => {
                // SAFETY: error_slot owns the only access to the optional output for this call.
                unsafe { error_slot.publish(error) }
            }
            Err(payload) => {
                // SAFETY: error_slot owns the only access to the optional output for this call.
                unsafe { error_slot.publish(panic_error(payload)) }
            }
        }
    }))
    .unwrap_or(ROFD_STATUS_INTERNAL)
}

/// Returns the status stored in an owned error handle.
///
/// A null handle returns [`ROFD_STATUS_INVALID_ARGUMENT`].
///
/// # Safety
///
/// A non-null `error` must be a live error handle returned by this library and
/// must not be freed concurrently.
#[no_mangle]
pub unsafe extern "C" fn rofd_error_get_status(error: *const rofd_error_t) -> rofd_status_t {
    catch_unwind(AssertUnwindSafe(|| {
        if error.is_null() {
            return ROFD_STATUS_INVALID_ARGUMENT;
        }
        // SAFETY: rofd_error_t has one sealed mapping to ErrorHandle.
        unsafe { handle_ref::<rofd_error_t>(error) }.status
    }))
    .unwrap_or(ROFD_STATUS_INTERNAL)
}

/// Returns the borrowed NUL-terminated UTF-8 message of an error handle.
///
/// A null handle returns null. The returned pointer remains valid only until
/// `error` is passed to [`rofd_error_free`]. It must not be freed separately.
///
/// # Safety
///
/// A non-null `error` must be a live error handle returned by this library and
/// must not be freed concurrently.
#[no_mangle]
pub unsafe extern "C" fn rofd_error_get_message(error: *const rofd_error_t) -> *const c_char {
    catch_unwind(AssertUnwindSafe(|| {
        if error.is_null() {
            return ptr::null();
        }
        // SAFETY: rofd_error_t has one sealed mapping to ErrorHandle.
        unsafe { handle_ref::<rofd_error_t>(error) }
            .message
            .as_ptr()
    }))
    .unwrap_or(ptr::null())
}

/// Frees an owned error handle. A null handle is a no-op.
///
/// # Safety
///
/// A non-null `error` must be a live error handle returned by this library,
/// owned by the caller, and not previously freed or used concurrently.
#[no_mangle]
pub unsafe extern "C" fn rofd_error_free(error: *mut rofd_error_t) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if error.is_null() {
            return;
        }
        // SAFETY: The caller transfers the unique ErrorHandle allocation back to its matching
        // sealed token mapping. drop_raw_handle reconstructs its Box exactly once.
        unsafe { drop_raw_handle::<rofd_error_t>(error) };
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handles::{drop_raw_handle, TestHandleStorage, TestHandleToken};
    use crate::{
        rofd_error_free, rofd_error_get_message, rofd_error_get_status, rofd_error_t,
        ROFD_STATUS_INTERNAL, ROFD_STATUS_INVALID_ARGUMENT, ROFD_STATUS_INVALID_DOCUMENT,
        ROFD_STATUS_IO, ROFD_STATUS_LIMIT_EXCEEDED, ROFD_STATUS_OUT_OF_MEMORY,
        ROFD_STATUS_PAGE_OUT_OF_RANGE, ROFD_STATUS_RENDER_ERROR, ROFD_STATUS_UNSUPPORTED,
    };
    use std::ffi::CStr;
    use std::panic::panic_any;
    use std::path::PathBuf;
    use std::ptr;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    #[repr(C, align(4))]
    struct WideScalar([u8; 8]);

    impl scalar_private::Sealed for WideScalar {}

    impl ZeroScalar for WideScalar {
        const ZERO: Self = Self([0; 8]);
    }

    #[test]
    fn embedded_nul_is_made_visible_without_changing_status() {
        let error = FfiError::new(ROFD_STATUS_RENDER_ERROR, "before\0after");
        assert_eq!(error.status(), ROFD_STATUS_RENDER_ERROR);
        assert_eq!(error.message().to_bytes(), b"before\\0after");
    }

    #[test]
    fn error_accessors_preserve_status_and_borrow_message_until_free() {
        let error = FfiError::new(ROFD_STATUS_IO, "read failed").into_raw();
        assert_eq!(unsafe { rofd_error_get_status(error) }, ROFD_STATUS_IO);
        let message = unsafe { rofd_error_get_message(error) };
        assert_eq!(
            unsafe { CStr::from_ptr(message) }.to_bytes(),
            b"read failed"
        );
        unsafe { rofd_error_free(error) };
    }

    #[test]
    fn boundary_catches_panic_after_initializing_outputs() {
        let mut output = ptr::dangling_mut::<TestHandleToken>();
        let mut error: *mut rofd_error_t = ptr::null_mut();

        let status = unsafe {
            boundary(
                &mut error,
                HandleOutput::<TestHandleToken>::required(&mut output),
                || panic!("ffi panic probe"),
            )
        };

        assert_eq!(status, ROFD_STATUS_INTERNAL);
        assert!(output.is_null());
        assert!(!error.is_null());
        let message = unsafe { CStr::from_ptr(rofd_error_get_message(error)) };
        assert!(message
            .to_bytes()
            .windows(15)
            .any(|part| part == b"ffi panic probe"));
        unsafe { rofd_error_free(error) };
    }

    #[test]
    fn boundary_clears_the_error_slot_on_success() {
        let mut old_error = ptr::dangling_mut::<rofd_error_t>();
        let status = unsafe { boundary(&mut old_error, (), || Ok(())) };
        assert_eq!(status, crate::ROFD_STATUS_OK);
        assert!(old_error.is_null());
    }

    #[test]
    fn all_outputs_are_initialized_before_a_missing_required_output_is_reported() {
        let mut later_handle = ptr::dangling_mut::<TestHandleToken>();
        let mut scalar = 99_usize;
        let mut called = false;
        let status = unsafe {
            boundary(
                ptr::null_mut(),
                (
                    HandleOutput::<TestHandleToken>::required(ptr::null_mut()),
                    HandleOutput::<TestHandleToken>::required(&mut later_handle),
                    ScalarOutput::required(&mut scalar),
                ),
                || {
                    called = true;
                    unreachable!("operation must not run with a missing required output")
                },
            )
        };
        assert_eq!(status, ROFD_STATUS_INVALID_ARGUMENT);
        assert!(!called);
        assert!(later_handle.is_null());
        assert_eq!(scalar, 0);
    }

    #[test]
    fn staged_owned_output_is_dropped_when_operation_panics() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut output = ptr::dangling_mut::<TestHandleToken>();
        let status = unsafe {
            boundary(
                ptr::null_mut(),
                HandleOutput::<TestHandleToken>::required(&mut output),
                || {
                    let staged = Box::new(TestHandleStorage::new(Arc::clone(&drops)));
                    assert_eq!(drops.load(Ordering::SeqCst), 0);
                    let _keep_owned_until_unwind = staged;
                    panic!("after staging owned output")
                },
            )
        };

        assert_eq!(status, ROFD_STATUS_INTERNAL);
        assert!(output.is_null());
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn non_string_panic_is_contained_and_described() {
        let mut error = ptr::null_mut();
        let status = unsafe {
            boundary(&mut error, (), || -> Result<(), FfiError> {
                panic_any(73_u8)
            })
        };

        assert_eq!(status, ROFD_STATUS_INTERNAL);
        assert!(!error.is_null());
        let message = unsafe { CStr::from_ptr(rofd_error_get_message(error)) };
        assert_eq!(
            message.to_bytes(),
            b"panic at FFI boundary: non-string panic payload"
        );
        unsafe { rofd_error_free(error) };
    }

    #[test]
    fn null_error_slot_accepts_failures_and_panics() {
        let failed = unsafe {
            boundary(ptr::null_mut(), (), || {
                Err(FfiError::new(ROFD_STATUS_IO, "probe"))
            })
        };
        let panicked = unsafe {
            boundary(ptr::null_mut(), (), || -> Result<(), FfiError> {
                panic!("probe")
            })
        };

        assert_eq!(failed, ROFD_STATUS_IO);
        assert_eq!(panicked, ROFD_STATUS_INTERNAL);
    }

    #[test]
    fn failure_replaces_a_prefilled_error_slot_with_a_new_error() {
        let sentinel = ptr::dangling_mut::<rofd_error_t>();
        let mut error = sentinel;
        let status = unsafe {
            boundary(&mut error, (), || {
                Err(FfiError::new(ROFD_STATUS_IO, "new error"))
            })
        };

        assert_eq!(status, ROFD_STATUS_IO);
        assert!(!error.is_null());
        assert_ne!(error, sentinel);
        assert_eq!(unsafe { rofd_error_get_status(error) }, ROFD_STATUS_IO);
        unsafe { rofd_error_free(error) };
    }

    #[test]
    fn aliased_handle_outputs_are_rejected_without_touching_them() {
        let sentinel = ptr::dangling_mut::<TestHandleToken>();
        let mut output = sentinel;
        let mut error = ptr::null_mut();
        let mut called = false;
        let status = unsafe {
            boundary(
                &mut error,
                (
                    HandleOutput::<TestHandleToken>::required(&mut output),
                    HandleOutput::<TestHandleToken>::required(&mut output),
                ),
                || {
                    called = true;
                    Err(FfiError::new(ROFD_STATUS_IO, "must not run"))
                },
            )
        };
        let error_status = unsafe { rofd_error_get_status(error) };
        unsafe { rofd_error_free(error) };

        assert_eq!(status, ROFD_STATUS_INVALID_ARGUMENT);
        assert_eq!(error_status, ROFD_STATUS_INVALID_ARGUMENT);
        assert!(!called);
        assert_eq!(output, sentinel);
    }

    #[test]
    fn partially_overlapping_scalar_outputs_are_rejected_without_touching_them() {
        #[repr(C, align(4))]
        struct AlignedBytes([u8; 12]);

        let mut bytes = AlignedBytes([0xa5; 12]);
        let first = bytes.0.as_mut_ptr().cast::<WideScalar>();
        // SAFETY: AlignedBytes has alignment 4 and this four-byte offset remains aligned for
        // WideScalar. The raw regions overlap, but no references are created from them.
        let second = unsafe { bytes.0.as_mut_ptr().add(4).cast::<WideScalar>() };
        let mut error = ptr::null_mut();
        let mut called = false;
        let status = unsafe {
            boundary(
                &mut error,
                (
                    ScalarOutput::required(first),
                    ScalarOutput::required(second),
                ),
                || {
                    called = true;
                    Err(FfiError::new(ROFD_STATUS_IO, "must not run"))
                },
            )
        };
        let error_status = unsafe { rofd_error_get_status(error) };
        unsafe { rofd_error_free(error) };

        assert_eq!(status, ROFD_STATUS_INVALID_ARGUMENT);
        assert_eq!(error_status, ROFD_STATUS_INVALID_ARGUMENT);
        assert!(!called);
        assert_eq!(bytes.0, [0xa5; 12]);
    }

    #[test]
    fn error_and_handle_output_alias_is_rejected_without_touching_either_slot() {
        let sentinel = ptr::dangling_mut::<rofd_error_t>();
        let mut shared = sentinel;
        let mut called = false;
        let status = unsafe {
            boundary(
                &mut shared,
                HandleOutput::<TestHandleToken>::required(
                    (&mut shared as *mut *mut rofd_error_t).cast(),
                ),
                || {
                    called = true;
                    Ok(Box::new(TestHandleStorage::new(Arc::new(
                        AtomicUsize::new(0),
                    ))))
                },
            )
        };

        if status == ROFD_STATUS_OK {
            // The pre-fix implementation published this test handle before the assertion.
            // Recover it through its sealed mapping so the intentional RED run did not leak.
            unsafe { drop_raw_handle::<TestHandleToken>(shared.cast()) };
        }
        assert_eq!(status, ROFD_STATUS_INVALID_ARGUMENT);
        assert!(!called);
        assert_eq!(shared, sentinel);
    }

    #[test]
    fn successful_handle_commit_uses_the_tokens_associated_storage_once() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut output = ptr::null_mut();
        let status = unsafe {
            boundary(
                ptr::null_mut(),
                HandleOutput::<TestHandleToken>::required(&mut output),
                || Ok(Box::new(TestHandleStorage::new(Arc::clone(&drops)))),
            )
        };

        assert_eq!(status, ROFD_STATUS_OK);
        assert!(!output.is_null());
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        unsafe { drop_raw_handle::<TestHandleToken>(output) };
        assert_eq!(drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn overflowing_output_address_range_is_rejected_before_any_write() {
        let aligned_overflowing_address = usize::MAX & !(align_of::<u64>() - 1);
        let output = aligned_overflowing_address as *mut u64;
        let mut called = false;
        let status = unsafe {
            boundary(ptr::null_mut(), ScalarOutput::required(output), || {
                called = true;
                Ok(1)
            })
        };

        assert_eq!(status, ROFD_STATUS_INVALID_ARGUMENT);
        assert!(!called);
    }

    #[test]
    fn core_errors_map_to_stable_statuses() {
        let io = rofd_core::Error::Io {
            path: PathBuf::from("sample.ofd"),
            source: std::io::Error::other("probe"),
        };
        assert_eq!(core_status(&io), ROFD_STATUS_IO);
        assert_eq!(
            core_status(&rofd_core::Error::LimitExceeded("probe".into())),
            ROFD_STATUS_LIMIT_EXCEEDED
        );
        assert_eq!(
            core_status(&rofd_core::Error::UnsupportedFeature("probe".into())),
            ROFD_STATUS_UNSUPPORTED
        );
        assert_eq!(
            core_status(&rofd_core::Error::PageOutOfRange {
                index: 3,
                page_count: 1,
            }),
            ROFD_STATUS_PAGE_OUT_OF_RANGE
        );
        assert_eq!(
            core_status(&rofd_core::Error::Container("probe".into())),
            ROFD_STATUS_INVALID_DOCUMENT
        );
        assert_eq!(
            core_status(&rofd_core::Error::InvalidStructure {
                path: "Document.xml".into(),
                message: "probe".into(),
            }),
            ROFD_STATUS_INVALID_DOCUMENT
        );
        assert_eq!(
            core_status(&rofd_core::Error::Internal(
                "page initialization lock is poisoned".into()
            )),
            ROFD_STATUS_INTERNAL
        );
    }

    #[test]
    fn render_errors_map_to_stable_statuses() {
        assert_eq!(
            render_status(&rofd_render::Error::InvalidOption {
                field: "dpi",
                value: "0".into(),
            }),
            ROFD_STATUS_INVALID_ARGUMENT
        );
        assert_eq!(
            render_status(&rofd_render::Error::RasterBudgetExceeded {
                required_bytes: 2,
                max_bytes: 1,
            }),
            ROFD_STATUS_LIMIT_EXCEEDED
        );
        assert_eq!(
            render_status(&rofd_render::Error::RasterAllocation { required_bytes: 2 }),
            ROFD_STATUS_OUT_OF_MEMORY
        );
        assert_eq!(
            render_status(&rofd_render::Error::UnsupportedDisplayCommand {
                command: rofd_render::DisplayCommandKind::Image,
            }),
            ROFD_STATUS_UNSUPPORTED
        );
        assert_eq!(
            render_status(&rofd_render::Error::ObjectResource {
                object_id: 7,
                resource_id: 8,
                kind: rofd_core::ResourceKind::Image,
                source: rofd_core::Error::LimitExceeded("probe".into()),
            }),
            ROFD_STATUS_LIMIT_EXCEEDED
        );
        assert_eq!(
            render_status(&rofd_render::Error::FontCache {
                message: "poisoned".into(),
            }),
            ROFD_STATUS_INTERNAL
        );
        assert_eq!(
            render_status(&rofd_render::Error::InvalidDisplayList {
                message: "probe".into(),
            }),
            ROFD_STATUS_RENDER_ERROR
        );
    }
}
