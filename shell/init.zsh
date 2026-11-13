# multigh interactive shell integration
if [[ -o interactive && -z ${_MGH_ZSH_LOADED:-} ]]; then
    _MGH_ZSH_LOADED=1
    export MGH_SHELL_SESSION="$$-$RANDOM-$RANDOM"
    if (( ! $+functions[compdef] )); then
        autoload -Uz compinit
        compinit -D
    fi
    eval "$(mgh shell completions zsh)"
    autoload -Uz add-zsh-hook
    _mgh_enter() {
        if [[ ${_MGH_LAST_PWD:-} != "$PWD" ]]; then
            _MGH_LAST_PWD=$PWD
            local _mgh_repo
            _mgh_repo=$(command git rev-parse --absolute-git-dir 2>/dev/null)
            if [[ ${_MGH_LAST_REPO:-} != "$_mgh_repo" ]]; then
                _MGH_LAST_REPO=$_mgh_repo
                mgh internal shell-enter
            fi
        fi
    }
    add-zsh-hook chpwd _mgh_enter
    _mgh_poll() { mgh internal poll; }
    add-zsh-hook precmd _mgh_poll
    if (( ! $+functions[git] && ! $+aliases[git] )); then
        function git { mgh internal git -- "$@"; }
    fi
    _MGH_LAST_PWD=$PWD
    _MGH_LAST_REPO=$(command git rev-parse --absolute-git-dir 2>/dev/null)
    mgh internal shell-enter --startup
fi
