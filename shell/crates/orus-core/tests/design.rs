use orus_core::design::SCALE;

#[test]
fn the_interface_is_drawn_at_eighty_percent_of_the_mock_up() {
    assert_eq!(SCALE, 0.8);
}
