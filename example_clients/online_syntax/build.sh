set -e

LIB_FOLDER="online-syntax-js"
FFI_FOLDER="online_syntax_ffi"

# assert we are in the same directory as this script
SCRIPT_DIR=$(dirname "$(readlink -f "$0")")
CUR_DIR=$(pwd)
if [ "$SCRIPT_DIR" != "$CUR_DIR" ]; then
  echo "Please run this script from the directory it is located in: $SCRIPT_DIR"
  exit 1
fi

# build #[wasm_bindgen] JS imports as basic JavaScript module into `${LIB_FOLDER}/wbg`
wasm-pack build -t web -d "../${LIB_FOLDER}/wbg" ${FFI_FOLDER}/

# run diplomat-tool for #[diplomat::bridge] modules
# TODO: remove the legacy config once the stable rust compiler switches to the C spec abi
diplomat-tool --config js.abi="legacy" -e ${FFI_FOLDER}/src/lib.rs js "${LIB_FOLDER}/api"

# fix diplomat generated code
cp ${LIB_FOLDER}/diplomat-wasm.mjs.template ${LIB_FOLDER}/api/diplomat-wasm.mjs

# fix wasm-bindgen generated code
# fix .gitignore
cp ${LIB_FOLDER}/wbg/.gitignore.template ${LIB_FOLDER}/wbg/.gitignore
# fix `${FFI_FOLDER}.js
file="${LIB_FOLDER}/wbg/${FFI_FOLDER}.js"

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