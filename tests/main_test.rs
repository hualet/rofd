use rofd;

#[test]
fn test_main() {
    let mut ofd_node = rofd::read_ofd("learning/test.ofd").unwrap();
    println!("ofd: {:?}", ofd_node);
    rofd::export_ofd_to_png(&mut ofd_node, "target/out.png").unwrap();
}
