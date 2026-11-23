use crate::value::position::Position;

#[test]
fn kanshi_round_trip() {
    let p = Position::new(-1920, 420);
    assert_eq!(Position::from_kanshi(&p.kanshi_token()), Some(p));
    assert_eq!(p.hypr_token(), "-1920x420");
}
