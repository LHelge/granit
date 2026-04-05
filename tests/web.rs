use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn pass() {
    let sum = 1 + 1;
    assert_eq!(sum, 2);
}
