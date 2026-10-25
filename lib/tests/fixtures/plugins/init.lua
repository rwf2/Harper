harper.register_filter("repeat_text", function(value, times)
    return string.rep(value, times)
end)

harper.register_function("record", function(value)
    return { value = value }
end)

harper.register_test("positive", function(value)
    return value > 0
end)
