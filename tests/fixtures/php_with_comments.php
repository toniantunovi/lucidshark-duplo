<?php

namespace App\Services;

// Single-line comment
use App\Models\User;

/**
 * PHPDoc comment block
 * @param string $input
 * @return array
 */
#[Route('/api/data')]
public function processData(string $input): array {
    // Compute step
    $x = computeValue($input);
    $y = transformData($x);
    $z = finalizeResult($y);
    $result = processOutput($z); /* inline comment */
    return $result;
}
