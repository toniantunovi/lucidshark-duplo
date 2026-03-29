require 'json'

# This is a comment
=begin
This is a block comment
spanning multiple lines
=end

def process_data(input)
    # Setup step
    x = compute_value(input)
    y = transform_data(x)
    z = finalize_result(y)
    result = process_output(z) # inline comment
    result
end
