package com.example

import scala.collection.mutable

@throws(classOf[Exception])
def processData(input: String): Result = {
    val x = computeValue(input)
    val y = transformData(x)
    val z = finalizeResult(y)
    val result = processOutput(z)
    result
}
