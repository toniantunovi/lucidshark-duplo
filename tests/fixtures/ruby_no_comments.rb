require 'json'

def process_data(input)
    x = compute_value(input)
    y = transform_data(x)
    z = finalize_result(y)
    result = process_output(z)
    result
end
