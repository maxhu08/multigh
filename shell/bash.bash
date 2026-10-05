__mgh_enter() {
    if [ "${__mgh_last_pwd-}" != "$PWD" ]; then
        __mgh_last_pwd="$PWD"
        command mgh internal enter
    fi
}

case $- in
    *i*)
        eval "$(command mgh shell completions bash)"

        if ! declare -F git >/dev/null && ! alias git >/dev/null 2>&1; then
            function git {
                case " $* " in
                    *' init '*) command mgh internal git -- "$@" ;;
                    *) command git "$@" ;;
                esac
            }
        fi

        if [[ $(declare -p PROMPT_COMMAND 2>/dev/null) == declare\ -a* ]]; then
            if [[ " ${PROMPT_COMMAND[*]} " != *" __mgh_enter "* ]]; then
                PROMPT_COMMAND+=(__mgh_enter)
            fi
        else
            case ";${PROMPT_COMMAND-};" in
                *';__mgh_enter;'*) ;;
                *) PROMPT_COMMAND="${PROMPT_COMMAND:+${PROMPT_COMMAND};}__mgh_enter" ;;
            esac
        fi
        ;;
esac
