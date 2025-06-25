set -e

# assert we are in the same directory as this script
SCRIPT_DIR=$(dirname "$(readlink -f "$0")")
CUR_DIR=$(pwd)
if [ "$SCRIPT_DIR" != "$CUR_DIR" ]; then
  echo "Please run this script from the directory it is located in: $SCRIPT_DIR"
  exit 1
fi

# build #[wasm_bindgen] JS imports as basic JavaScript module into `simple-semantics-js/wbg`
wasm-pack build -t web -d "../simple-semantics-js/wbg" simple_semantics_ffi/

# run diplomat-tool for #[diplomat::bridge] modules
# TODO: remove the legacy config once the stable rust compiler switches to the C spec abi
diplomat-tool --config js.abi="legacy" -e simple_semantics_ffi/src/lib.rs js "simple-semantics-js/api"

# fix diplomat generated code
cp simple-semantics-js/diplomat-wasm.mjs.template simple-semantics-js/api/diplomat-wasm.mjs

# fix wasm-bindgen generated code
# fix .gitignore
cp simple-semantics-js/wbg/.gitignore.template simple-semantics-js/wbg/.gitignore
# fix `simple_semantics_ffi.js
file="simple-semantics-js/wbg/simple_semantics_ffi.js"

# Remove the line: import * as __wbg_star0 from 'env';
sed -i "/^import \* as __wbg_star0 from 'env';$/d" "$file"

# Change "function __wbg_get_imports()" back to "export function __wbg_get_imports()"
sed -i "s/^function __wbg_get_imports()/export function __wbg_get_imports()/" "$file"

# Remove the line: imports['env'] = __wbg_star0;
sed -i "/imports\['env'\] = __wbg_star0;/d" "$file"

# Change "async function __wbg_init(module_or_path)" back to "async function __wbg_init(imports, module_or_path)"
sed -i "s/^async function __wbg_init(module_or_path)/async function __wbg_init(imports, module_or_path)/" "$file"
sed -i "s/^function initSync(module) {/function initSync(imports, module) {/" "$file"

# Remove the line: const imports = __wbg_get_imports();
sed -i "/^    const imports = __wbg_get_imports();$/d" "$file"