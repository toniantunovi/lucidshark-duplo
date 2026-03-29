package com.example

// Single-line comment
import scala.collection.mutable

/**
 * Scaladoc comment block
 * @param input the input string
 * @return processed result
 */
@throws(classOf[Exception])
def processData(input: String): Result = {
    // Compute step
    val x = computeValue(input)
    val y = transformData(x)
    val z = finalizeResult(y)
    val result = processOutput(z) /* inline comment */
    result
}
