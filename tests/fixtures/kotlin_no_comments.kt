package com.example

import kotlin.collections.List

@JvmStatic
fun processData(input: String): Result {
    val x = computeValue(input)
    val y = transformData(x)
    val z = finalizeResult(y)
    val result = processOutput(z)
    return result
}
