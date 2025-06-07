# eggstraction 🪺⚡  
*Cost-Aware Formal Optimization of Finite-Field Arithmetic*

> **CS 292C / UCSB — June 2025**  
> Matthew • Praneeth

---

## ✨ What is it?

`eggstraction` is a research prototype that **automatically rewrites finite-field expressions to lower cost on a given platform**, while _proving_ the optimized code is semantically equivalent to the original.

* **Equality-Saturation Core** – powered by the [`egg`](https://github.com/egraphs-good/egg) e-graph library.  
* **DAG-Aware Extraction** – replaces greedy tree extraction with an **ILP formulation** solved by the open-source **CBC** solver.  
* **Pluggable Cost Model** – read from JSON so you can dial costs for `mul`, `sq`, `const_mul`, … to match your hardware.  
* **Tower-Field “Full Search”** – optional Python wrapper explores quadratic extensions (e.g. 𝔽<sub>p⁶</sub>, 𝔽<sub>p¹²</sub>) automatically.

The result:  
> > *31 → 26* operations on the running example, a 16 % reduction — and the proof is in the e-graph!

Slides of the full approach are in `docs/slides.pdf`.

---

## 🚀 Quick start with Docker

> Docker 24.x or newer is fine.

### 1 — Build the image

```git clone https://github.com/realmatthewpeng/eggstraction.git
cd eggstraction
docker build -t eggstraction .
```

### 2 — Run the optimizer

```
# assumes the three input files live in the current directory
docker run --rm eggstraction
```

#### Inputs

| Flag(s)                | Argument | Purpose                                   | Default             |
| ---------------------- | -------- | ----------------------------------------- | ------------------- |
| `-h`, `--help`         | —        | Show help and exit                        | —                   |
| `-t`, `--tests`        | *FILE*   | Path to test-case list                    | `tests.txt`         |
| `-c`, `--cost_model`   | *FILE*   | Path to JSON cost model                   | `cost_model.json`   |
| `-s`, `--symbol_types` | *FILE*   | Path to JSON symbol-type map              | `symbol_types.json` |
| `-f`, `--full_search`  | —        | Enable quadratic-tower “full search” mode | off                 |



#### Examples

Run with custom paths:
```
docker run --rm -v $(pwd):/inputs eggstraction \
  -t /inputs/my_tests.txt \
  -c /inputs/montgomery_costs.json \
  -s /inputs/bn254_symbols.json
```


## Happy optimizing!