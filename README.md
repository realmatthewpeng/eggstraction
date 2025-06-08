# eggstraction 🪺⚡  
*Cost-Aware Formal Optimization of Finite-Field Arithmetic*

> **CS 292C / UCSB — June 2025**  
> Matthew • Praneeth

---

## ✨ What is it?

`eggstraction` is a research prototype that **automatically rewrites finite-field expressions to lower cost on a given platform**, while _guaranteeing_ that the optimized code is semantically equivalent to the original.

* **Equality-Saturation Core** – powered by the [`egg`](https://github.com/egraphs-good/egg) e-graph library.  
* **DAG-Aware Extraction** – replaces greedy tree extraction with an **ILP formulation** solved by the open-source **CBC** solver.  
* **Customizable Cost Model** – read from JSON so you can dial costs for `mul`, `sq`, `const_mul`, … to match your hardware.  
* **Towering-Field “Full Search”** – optional Python wrapper explores quadratic extensions (e.g. 𝔽<sub>p⁴</sub> → 𝔽<sub>p²</sub>) automatically.

The result:  
> On the motivating example, `eggstraction` optimizes the cost from *31 → 26*, a 16% reduction!

Slides of the full approach are in `docs/slides.pdf`.

---

## 🚀 Quick start with Docker

> Docker 24.x or newer is fine.

### 1 — Build the image

```bash
git clone https://github.com/realmatthewpeng/eggstraction.git
cd eggstraction
docker build -t eggstraction .
```

### 2 — Run the optimizer

```bash
# Shows the result for the motiviating example
docker run --rm eggstraction
```

#### Command Line Options

| Flag(s)                | Argument | Purpose                                   | Default                    |
| ---------------------- | -------- | ----------------------------------------- | -------------------------- |
| `-h`, `--help`         | —        | Show help and exit                        | —                          |
| `-t`, `--tests`        | *FILE*   | Path to test-case list                    | `inputs/tests.txt`         |
| `-c`, `--cost_model`   | *FILE*   | Path to JSON cost model                   | `inputs/cost_model.json`   |
| `-s`, `--symbol_types` | *FILE*   | Path to JSON symbol-type map              | `inputs/symbol_types.json` |
| `-f`, `--full_search`  | —        | Enable quadratic-tower “full search” mode | off                        |

#### Benchmarks

The benchmarks we mention in our presentation can be found in `inputs/benchmarks.txt`. To replicate our results, copy the benchmark program into `inputs/tests.txt` and modify the cost model and symbol types JSON accordingly. Then, run the following command:

```bash
docker run --rm -v $(pwd)/inputs:/inputs eggstraction \
  -t /inputs/tests.txt \
  -c /inputs/cost_model.json \
  -s /inputs/symbol_types.json
```

Please note that some benchmarks should be ran with the `-f` flag, in which case just add `-f` to the end of the above command. 

## Happy Optimizing!
