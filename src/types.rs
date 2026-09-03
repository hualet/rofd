pub fn mmtopx(mm: f64) -> f64 {
    // TODO(hualet): use actual dpi?
    let dpi = 97.0;
    return mm * dpi / 25.4;
}

pub mod st {
    pub type Array = Vec<i32>;
}

pub mod ct {
    use crate::types::mmtopx;
    use crate::types::st;

    pub struct PageArea {
        pub x: f64,
        pub y: f64,
        pub width: f64,
        pub height: f64,
    }

    // Boundary="10.30 30.30 0.30 22"
    #[derive(Debug)]
    pub struct Box {
        pub x: f64,
        pub y: f64,
        pub width: f64,
        pub height: f64,
    }

    #[derive(Debug)]
    pub struct Color {
        pub value: st::Array,
        pub alpha: i32,
    }

    #[derive(Debug)]
    pub struct Matrix {
        pub a: f64,
        pub b: f64,
        pub c: f64,
        pub d: f64,
        pub e: f64,
        pub f: f64,
    }

    // implement string to PageArea
    impl From<String> for PageArea {
        fn from(value: String) -> Self {
            let parts: Vec<f64> = value
                .split_whitespace()
                .map(|s| s.parse().unwrap())
                .collect();
            PageArea {
                x: parts[0],
                y: parts[1],
                width: parts[2],
                height: parts[3],
            }
        }
    }

    impl PageArea {
        pub fn to_pixel(&self) -> PageArea {
            PageArea {
                x: mmtopx(self.x),
                y: mmtopx(self.y),
                width: mmtopx(self.width),
                height: mmtopx(self.height),
            }
        }
    }

    impl From<String> for Matrix {
        fn from(value: String) -> Self {
            let parts: Vec<f64> = value
                .split_whitespace()
                .map(|s| s.parse().unwrap())
                .collect();
            // According to the spec, the matrix should be 6 elements long.
            assert_eq!(parts.len(), 6);

            Matrix {
                a: parts[0],
                b: parts[1],
                c: parts[2],
                d: parts[3],
                e: parts[4],
                f: parts[5],
            }
        }
    }

    // implement string to Box
    impl From<String> for Box {
        fn from(value: String) -> Self {
            let parts: Vec<f64> = value
                .split_whitespace()
                .map(|s| s.parse().unwrap())
                .collect();
            Box {
                x: parts[0],
                y: parts[1],
                width: parts[2],
                height: parts[3],
            }
        }
    }

    impl Box {
        pub fn to_pixel(&self) -> Box {
            Box {
                x: mmtopx(self.x),
                y: mmtopx(self.y),
                width: mmtopx(self.width),
                height: mmtopx(self.height),
            }
        }
    }

    // implement string to Color
    impl From<String> for Color {
        fn from(value: String) -> Self {
            let parts: Vec<i32> = value
                .split_whitespace()
                .map(|s| s.parse().unwrap())
                .collect();
            Color {
                value: parts.clone(),
                alpha: 255,
            }
        }
    }
}
