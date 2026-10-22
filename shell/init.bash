# multigh interactive shell integration
if [[ $- == *i* ]]; then
    if [[ -z ${_MGH_BASH_LOADED:-} ]]; then
        _MGH_BASH_LOADED=1
        eval "$(mgh shell completions bash)"
        _mgh_enter() {
            if [[ ${_MGH_LAST_PWD:-} != "$PWD" ]]; then
                _MGH_LAST_PWD=$PWD
                mgh internal enter
            fi
        }
        if ! declare -F git >/dev/null && ! alias git >/dev/null 2>&1; then
            function git { mgh internal git -- "$@"; }
        fi
        _mgh_restore_status() { return "$1"; }
        _mgh_prompt_dispatch() {
            local _mgh_status=$? _mgh_handler
            for _mgh_handler in "${_MGH_PROMPT_HANDLERS[@]}"; do
                _mgh_restore_status "$_mgh_status"
                eval "$_mgh_handler"
                _mgh_status=$?
            done
            _mgh_enter
        }
        if [[ $(declare -p PROMPT_COMMAND 2>/dev/null) == 'declare -a'* ]]; then
            if (( BASH_VERSINFO[0] > 5 || (BASH_VERSINFO[0] == 5 && BASH_VERSINFO[1] >= 1) )); then
                PROMPT_COMMAND+=(_mgh_enter)
            else
                _MGH_PROMPT_HANDLERS=("${PROMPT_COMMAND[@]}")
                PROMPT_COMMAND[0]=_mgh_prompt_dispatch
            fi
        elif [[ -n ${PROMPT_COMMAND:-} ]]; then
            _MGH_PROMPT_HANDLERS=("$PROMPT_COMMAND")
            PROMPT_COMMAND=_mgh_prompt_dispatch
        else
            PROMPT_COMMAND=_mgh_enter
        fi
        mgh internal welcome
        _mgh_enter
    fi
fi
