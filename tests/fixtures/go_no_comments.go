package main

import "fmt"

func main() {
	x := computeValue()
	y := transformData(x)
	z := finalizeResult(y)
	result := processOutput(z)
	fmt.Println(result)
}
