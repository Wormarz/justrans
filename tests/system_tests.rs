use assert_cmd::Command;

#[test]
fn test_main() {
    let mut cmd = Command::cargo_bin("template").unwrap();
    cmd.env("RUST_LOG", "info");
    let output = cmd.output().unwrap();
    println!("{:?}", output);
    assert!(output.status.success());
}
