package com.example

// Single-line comment
import kotlin.collections.List

/**
 * KDoc comment block
 * @param input the input string
 */
@JvmStatic
fun processData(input: String): Result {
    // Compute step
    val x = computeValue(input)
    val y = transformData(x)
    val z = finalizeResult(y)
    val result = processOutput(z)
    return result /* inline comment */
}
