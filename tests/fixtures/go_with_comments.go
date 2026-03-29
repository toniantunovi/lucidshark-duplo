package main

// This is a single-line comment
import "fmt"

/*
 * This is a block comment
 * spanning multiple lines
 */

func main() {
    // Setup variables
    x := computeValue()
    y := transformData(x)
    z := finalizeResult(y)
    result := processOutput(z)
    fmt.Println(result) /* inline comment */
}
