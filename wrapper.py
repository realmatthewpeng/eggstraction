import subprocess
import re
import argparse
import json
import os

def parse_sexpr(expr):
    tokens = expr.replace('(', ' ( ').replace(')', ' ) ').split()
    def parse(tokens):
        if not tokens:
            raise SyntaxError("Unexpected EOF")
        token = tokens.pop(0)
        if token == '(':
            L = []
            while tokens[0] != ')':
                L.append(parse(tokens))
            tokens.pop(0)  # pop off ')'
            return L
        elif token == ')':
            raise SyntaxError("Unexpected )")
        else:
            return token
    return parse(tokens)

def traverse_ast(ast, symbols, depth=0):
    if isinstance(ast, list):
        for i in range(1, len(ast)):
            child = ast[i]
            if isinstance(child, list):
                traverse_ast(child, symbols, depth + 1)
            else:
                if child in symbols:
                    field = symbols[child]
                    if re.match(r"fp\d+", field):
                        field_value = int(field[2:])
                        if field_value > 2:
                            new_field = f"fp{field_value - 2}"
                        else:
                            new_field = "fp"
                        new_sym1 = "t" + child
                        new_sym2 = "tu" + child
                        new_pair = ["pair", new_sym1, new_sym2]
                        symbols[new_sym1] = new_field
                        symbols[new_sym2] = new_field
                        # print(f"replaced {child} with {new_pair}")
                        ast[i] = new_pair

def ast_to_sexpr(ast):
    if isinstance(ast, list):
        return '(' + ' '.join(ast_to_sexpr(child) for child in ast) + ')'
    else:
        return str(ast)
    
def run_optimizer(test_case_file, cost_model_file, symbol_types_file):
    # Build and run the Rust project using Cargo
    result = subprocess.run(
        ["cargo", "run", "--", symbol_types_file, cost_model_file, test_case_file],
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

# Set up argparse for optional arguments
parser = argparse.ArgumentParser(description="Run optimizer with custom cost and symbol files.")
parser.add_argument("-t", "--tests", default="tests.txt", help="Path to tests txt file")
parser.add_argument("-c", "--cost_model", default="cost_model.json", help="Path to cost model JSON file")
parser.add_argument("-s", "--symbol_types", default="symbol_types.json", help="Path to symbol types JSON file")
args = parser.parse_args()

test_case_file = args.tests
cost_model_file = args.cost_model
symbol_types_file = args.symbol_types

with open(test_case_file, "r") as f:
    with open(symbol_types_file, "r") as stf:
        symbol_types = json.load(stf)

    test_cases = [line.strip() for line in f if line.strip()]
    for i, test_case in enumerate(test_cases, start=0):
        # print(f"--- Test Case {i} ---")
        ast = parse_sexpr(test_case)
        # print(ast)
        traverse_ast(ast, symbol_types)
        new_expr = ast_to_sexpr(ast)
        # print(new_expr)
        # print(test_case)
        os.makedirs("tmp", exist_ok=True)

        with open("tmp/symbol_types_file", "w") as symf:
            json.dump(symbol_types, symf, indent=2)
        with open("tmp/test_case_file", "w") as tmpf:
            tmpf.write(test_case + "\n")
            tmpf.write(new_expr + "\n")
        
        run_optimizer("tmp/test_case_file", cost_model_file, "tmp/symbol_types_file")
