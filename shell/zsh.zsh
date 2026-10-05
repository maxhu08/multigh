__mgh_enter() {
    command mgh internal enter
}

if [[ -o interactive ]]; then
    if (( ! $+functions[git] && ! $+aliases[git] )); then
        function git {
            case " $* " in
                *' init '*) command mgh internal git -- "$@" ;;
                *) command git "$@" ;;
            esac
        }
    fi

    autoload -Uz add-zsh-hook
    add-zsh-hook chpwd __mgh_enter
    __mgh_enter
fi
