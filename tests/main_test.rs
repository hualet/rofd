use rofd::*;

#[test]
fn test_read_ofd() {
    let ofd_node = read_ofd("./learning/test.ofd").unwrap();
    println!("ofd: {:#?}", ofd_node);
}

#[test]
fn test_export_ofd_to_png() {
    let mut ofd_node = read_ofd("./learning/test.ofd").unwrap();
    println!("ofd: {:#?}", ofd_node);
    export_ofd_to_png(&mut ofd_node, "target/out.png", 400, 400).unwrap();
}
