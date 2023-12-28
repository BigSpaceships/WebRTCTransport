apt-get update
apt-get install -y \
    pkg-config

## Install rustup and common components
curl https://sh.rustup.rs -sSf | sh -s -- -y 

source "$HOME/.cargo/env"

curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
