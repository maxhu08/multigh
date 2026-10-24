# multigh interactive shell integration
if status is-interactive
    if not set -q _MGH_FISH_LOADED
        set -g _MGH_FISH_LOADED 1
        mgh shell completions fish | source
        function _mgh_enter --on-variable PWD
            if not set -q _MGH_LAST_PWD; or test "$_MGH_LAST_PWD" != "$PWD"
                set -g _MGH_LAST_PWD "$PWD"
                set -l _mgh_repo (command git rev-parse --absolute-git-dir 2>/dev/null)
                if not set -q _MGH_LAST_REPO; or test "$_MGH_LAST_REPO" != "$_mgh_repo"
                    set -g _MGH_LAST_REPO "$_mgh_repo"
                    if test -n "$_mgh_repo"
                        mgh internal enter </dev/tty
                    end
                end
            end
        end
        if not functions -q git
            function git
                mgh internal git -- $argv
            end
        end
        function _mgh_startup --on-event fish_prompt
            functions --erase _mgh_startup
            set -g _MGH_LAST_PWD "$PWD"
            set -g _MGH_LAST_REPO (command git rev-parse --absolute-git-dir 2>/dev/null)
            mgh internal startup </dev/tty
        end
    end
end
