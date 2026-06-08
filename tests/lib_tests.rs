use qubit_io_text::{
    CodingErrorPolicy,
    LineEnding,
};

#[test]
fn test_public_value_defaults_are_stable() {
    assert_eq!(LineEnding::Lf, LineEnding::default());
    assert_eq!("\n", LineEnding::Lf.as_str());
    assert_eq!("\r\n", LineEnding::CrLf.as_str());
    assert_eq!("\r", LineEnding::Cr.as_str());
    assert_eq!(CodingErrorPolicy::Strict, CodingErrorPolicy::default());
}
