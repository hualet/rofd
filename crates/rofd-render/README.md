# rofd-render

`rofd-render` lowers validated, GUI-independent `rofd-core` page objects into an
immutable backend-neutral display list. It does not read OFD archives or XML;
rendering backends consume its commands in order. Effective page layers already
include recursively resolved background and foreground templates, so display
commands and diagnostics preserve the merged paint order without introducing a
dependency from `rofd-core` back to the renderer.
