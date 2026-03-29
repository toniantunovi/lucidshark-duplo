import Foundation

@available(iOS 15, *)
func processData(input: String) -> Result<Data, Error> {
    let x = computeValue(input)
    let y = transformData(x)
    let z = finalizeResult(y)
    let result = processOutput(z)
    return result
}
