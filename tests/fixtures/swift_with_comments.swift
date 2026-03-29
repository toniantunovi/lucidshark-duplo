import Foundation

// Single-line comment

/**
 * Documentation comment
 * - Parameter input: the input string
 * - Returns: processed result
 */
@available(iOS 15, *)
func processData(input: String) -> Result<Data, Error> {
    // Compute step
    let x = computeValue(input)
    let y = transformData(x)
    let z = finalizeResult(y)
    let result = processOutput(z) /* inline comment */
    return result
}
