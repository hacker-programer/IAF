import sys
with open('src/agent.rs','rb') as f:
    data = f.read()
idx = data.find(b'"finalizar_tarea"')
print(f'Offset: {idx}')
print(repr(data[idx:idx+500]))
