function __mgh_enter --on-variable PWD
    if status is-interactive; and command -sq mgh
        command mgh internal enter 0<&1
    end
end

if status is-interactive
    command mgh shell completions fish | source

    if not functions -q git
        function git
            if contains -- init $argv
                command mgh internal git -- $argv 0<&1
            else
                command git $argv
            end
        end
    end

    __mgh_enter
end
