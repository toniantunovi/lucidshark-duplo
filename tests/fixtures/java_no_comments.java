package com.example;

import java.util.List;

@Override
public Result processData(String input) {
    int x = computeValue(input);
    int y = transformData(x);
    int z = finalizeResult(y);
    int result = processOutput(z);
    return result;
}
