__mgh_enter() {
    command mgh enter
}

if [[ -o interactive ]]; then
    autoload -Uz add-zsh-hook
    add-zsh-hook chpwd __mgh_enter
    __mgh_enter
fi
