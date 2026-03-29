package com.example;

// Single-line comment
import java.util.List;

/**
 * Javadoc comment block
 * @param input the input string
 * @return processed result
 */
@Override
public Result processData(String input) {
    // Compute step
    int x = computeValue(input);
    int y = transformData(x);
    int z = finalizeResult(y);
    int result = processOutput(z); /* inline comment */
    return result;
}
