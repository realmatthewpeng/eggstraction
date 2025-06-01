import subprocess
import re

# Build and run the Rust project using Cargo
result = subprocess.run(
    ["cargo", "run"],
    capture_output=True,
    text=True
)

parts = result.stdout.split("Optimizing_Test_Case ")

for i, test_output in enumerate(parts[1:], start=1):
    print(f"--- Test {i} ---")

    has_results = re.search(r'>>>(.*?)<<<', test_output, re.DOTALL)
    if has_results:
        print(has_results.group(1).strip() + "\n")
    else:
        print("Possible Error: No results found!")
        break

if result.returncode != 0:
    print("stderr output:")
    print(result.stderr)