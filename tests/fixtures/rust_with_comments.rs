/// Doc comment for the function
/// with multiple lines
fn calculate_sum(a: i32, b: i32) -> i32 {
    // Single line comment
    let result = a + b; /* inline */
    result
}

/* Block comment
   spanning lines */
fn calculate_product(a: i32, b: i32) -> i32 {
    /* /* nested */ comment */
    let result = a * b;
    result
}
