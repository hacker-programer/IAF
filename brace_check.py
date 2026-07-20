import sys

with open('orig_agent.rs', 'rb') as f:
    orig = f.read()

with open('src/agent.rs', 'rb') as f:
    curr = f.read()

print(f"Original: opens={orig.count(b'{')}, closes={orig.count(b'}')}, diff={orig.count(b'{') - orig.count(b'}')}")
print(f"Current:  opens={curr.count(b'{')}, closes={curr.count(b'}')}, diff={curr.count(b'{') - curr.count(b'}')}")
print(f"Lines: orig={orig.count(b'\n')}, curr={curr.count(b'\n')}")

# Find the extra opens
if curr.count(b'{') > curr.count(b'}'):
    print(f"\nThere are {curr.count(b'{') - curr.count(b'}')} unclosed braces in current")
