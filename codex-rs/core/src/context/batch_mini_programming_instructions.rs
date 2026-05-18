use super::ContextualUserFragment;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct BatchMiniProgrammingInstructions;

impl ContextualUserFragment for BatchMiniProgrammingInstructions {
    const ROLE: &'static str = "developer";
    const START_MARKER: &'static str = "<batch_mini_programming_instructions>";
    const END_MARKER: &'static str = "</batch_mini_programming_instructions>";

    fn body(&self) -> String {
        r#"`workflow_batch` tool is available, and it is Codex's default command-free batch mini-programming surface for dependent deterministic local workflows.
Use it proactively; do not wait for the user to mention batching or name the tool. For deterministic create/edit/verify file tasks that fit root-confined file/JSON IO, edits, assertions, conditions, loops, reductions, file metadata/listing, or other collection transformations, choose `workflow_batch` before shell commands, patch tools, or separate file-edit tool calls.
Use it aggressively for multi-step local workflows with dependent steps or repeated file/JSON operations that would otherwise require several separate tool calls without user input. Use normal focused tools such as shell/rg for single read-only probes, one-off searches, and unbounded repo-wide scans where streaming output is shorter and easier to inspect. Use `workflow_batch` for bounded recursive conditional scans when the workflow chains file listing, filtering, metadata, reads, assertions, transforms, or generated outputs.

Top-level tool arguments: provide exactly one of inline `spec` JSON or `spec_path`; optional `workdir`, `report_path`, and `log_path` keep execution root-confined and produce compact artifacts.
Spec shape: `{"steps":[...]}`. Each step is an object with optional `id` and optional `if`; branch steps may use `then` and `else`.

Step syntax reference:
- `set` or `set_vars`: variable map, e.g. `{"set":{"nums":[2,3,5],"summary":{"literal":{"sum":10,"product":30}}}}`.
- `ensure_dir`: create a directory, e.g. `{"ensure_dir":{"path":"output"}}`.
- `stat_path`: metadata/Test-Path replacement, e.g. `{"stat_path":{"path":"output/summary.json","var":"summary_stat","sha1":true}}`; returns `{exists,path,kind,is_file,is_dir,len,modified_unix,sha1?}`.
- `list_files`: constrained Get-ChildItem replacement, e.g. `{"list_files":{"path":"output","recursive":true,"pattern":"\\.json$","var":"json_paths"}}`; optional `include_dirs`, `details`, and `max_entries`.
- `read_file`: read text into a variable, e.g. `{"read_file":{"path":"output/readme.txt","var":"readme"}}`.
- `read_json`: parse JSON into a variable, e.g. `{"read_json":{"path":"output/summary.json","var":"summary"}}`.
- `write_file` and `append_file`: write text from `content`, e.g. `{"write_file":{"path":"output/readme.txt","content":"sum=10\nproduct=30"}}`.
- `write_json`: serialize expression `value`, e.g. `{"write_json":{"path":"output/summary.json","value":{"literal":{"sum":10,"product":30}}}}`.
- `copy_file`: copy root-confined files, e.g. `{"copy_file":{"from":"input/a.txt","to":"output/a.txt"}}`.
- `edit_file`: apply text operations, e.g. `{"edit_file":{"path":"notes.txt","operations":[{"op":"replace","pattern":"old","content":"new"}]}}`; edit ops include `insert_at_line`, `insert_at_position`, `replace_span`, `insert_before`, `insert_after`, and `replace`.
- `assert`: verify an expression, e.g. `{"assert":{"expr":{"eq":[{"ref":"summary"},{"literal":{"sum":10,"product":30}}]},"message":"summary mismatch"}}`.
- `for_each`: iterate with `items`, optional `as`, and nested `steps`, e.g. `{"for_each":{"ref":"json_paths"},"as":"path","steps":[...]}`.
- `while`: repeat while a condition is truthy, e.g. `{"while":{"lt":[{"ref":"i"},3]},"steps":[...]}`.
`run` exists in the generic workflow runner but is disabled for this tool; command execution remains on the normal approval path, outside this batch surface.

PowerShell substitutions: use `stat_path` instead of `Test-Path`/`Get-Item`, `list_files` instead of constrained `Get-ChildItem`, `ensure_dir` instead of `New-Item -ItemType Directory`, `read_file`/`write_file`/`append_file` instead of `Get-Content`/`Set-Content`/`Add-Content`, and `read_json`/`write_json` plus expressions instead of `ConvertFrom-Json`/`ConvertTo-Json`.

Expression/composite types: expressions are JSON values. Scalars, arrays, and objects are supported; variables can hold strings, numbers, booleans, null, arrays, and JSON objects. JSON objects are the map/dictionary/hashmap/record/struct type: keys are strings and values are arbitrary JSON. Use refs as `{"ref":"vars.name"}` or `{"ref":"steps.step_id"}`; bare variable refs like `{"ref":"name"}` are also accepted. There is no dedicated hashset type; represent sets as arrays and use `unique`, `set_union`, `set_intersection`, `set_difference`, and `set_includes`. Because objects are also used for expression operators, wrap object constants with `{"literal":{...}}`.
Expression keywords: `literal`, `ref`, `lines`, `strip`, `len`, `range`, `split`, `sort`, `unique`, `take`, `get`, `join`, `map`, `filter`, `reduce`, `all_of`, `any_of`, `none_of`, `count_if`, `find_if`, `partition`, `group_by`, `scan`, `set_union`, `set_intersection`, `set_difference`, `set_includes`, `min`, `max`, `enumerate`, `zip`, `parse_json`, `to_json`, `keys`, `values`, `entries`, `from_entries`, `merge`, `pick`, `omit`, `not`, `and`, `or`, `contains`, `starts_with`, `ends_with`, `matches`, `add`, `sub`, `eq`, `ne`, `lt`, `lte`, `gt`, and `gte`.
Functional collection usage: use `map`, `filter`, `reduce`, `scan`, quantifiers, set operations, and object operations inside `set`, `write_json`, `assert`, or loop conditions instead of spilling intermediate files or shelling out. Example: `{"set":{"plus_one":{"map":{"items":{"ref":"nums"},"as":"n","expr":{"add":[{"ref":"n"},1]}}}}}`.
Composite example: `{"set":{"record":{"literal":{"name":"fixture","items":[2,3,5]}},"item_count":{"len":{"get":{"from":{"ref":"record"},"key":"items"}}}}}`.
Assertion usage: prefer structured JSON expressions and `message` diagnostics. Avoid free-form prose assertions.
Prefer inline `spec` for one-shot batches. Use `spec_path` only for reusable, large, or already-existing canaries.
Keep batches compact: include only the necessary dependent steps, concise variables, assertions or early exits, and rely on the compact tool summary plus report/log artifacts. Split when an operation needs user input, command execution, crosses the active permission boundary, is destructive or irreversible, or would be materially harder to diagnose if batched."#
            .to_string()
    }
}
