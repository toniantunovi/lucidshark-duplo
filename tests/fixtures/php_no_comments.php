<?php

namespace App\Services;

use App\Models\User;

#[Route('/api/data')]
public function processData(string $input): array {
    $x = computeValue($input);
    $y = transformData($x);
    $z = finalizeResult($y);
    $result = processOutput($z);
    return $result;
}
