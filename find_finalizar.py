import sys
with open('src/agent.rs','rb') as f:
    data = f.read()
# Skip first occurrence (tool definition)
idx1 = data.find(b'"finalizar_tarea"')
print(f'First at: {idx1}')
idx2 = data.find(b'"finalizar_tarea"', idx1 + 1)
print(f'Second at: {idx2}')
if idx2 > 0:
    chunk = data[idx2:idx2+400]
    print(repr(chunk))
    # Also check: does it start with spaces then "finalizar_tarea"?
    # The handler is:                     "finalizar_tarea" => {
    # Let's find the exact pattern
    # Also search for: \n                    "finalizar_tarea
    pattern = b'\n                    "finalizar_tarea"'
    idx3 = data.find(pattern)
    print(f'\nHandler at: {idx3}')
    if idx3 > 0:
        print(repr(data[idx3:idx3+500]))
