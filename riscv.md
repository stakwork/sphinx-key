
install the gcc risv toolchain on macos:

https://github.com/riscv-collab/riscv-gnu-toolchain

```
git clone --recursive https://github.com/riscv/riscv-gnu-toolchain

cd riscv-gnu-toolchain

./configure --prefix=/opt/riscv

sudo make

export PATH=$PATH:/opt/riscv/bin

(add that line above to your ~/.bash_profile to make it stick)
```

OR maybe this is better? 

https://github.com/riscv-software-src/homebrew-riscv

```
brew tap riscv-software-src/riscv

brew install riscv-tools
```
