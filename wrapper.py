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

    dag_costs = re.findall(r'Objective value:\s+(\d+\.\d+)', test_output)
    dag_costs = [int(float(val)) for val in dag_costs]

    has_results = re.search(r'>>>(.*?)<<<', test_output, re.DOTALL)
    if has_results:
        results = has_results.group(0).split("\n")
        results.insert(-2, "DAG: Initial cost    : " + str(dag_costs[0]))
        results.insert(-1, "DAG: Optimized cost  : " + str(dag_costs[1]) + "\n")
        for data in results[1:-1]:
            print(data)
    else:
        raise RuntimeError("No match found.")

if result.returncode != 0:
    print("Rust program errors:")
    print(result.stderr)