# multigh interactive shell integration for Windows PowerShell 5.1 and PowerShell 7
& {
    if ((Get-Variable _MghPowerShellLoaded -Scope Global -ErrorAction Ignore) -or
        [Environment]::OSVersion.Platform -ne 'Win32NT' -or
        $Host.Name -ne 'ConsoleHost' -or [Console]::IsInputRedirected -or
        [Console]::IsOutputRedirected -or [Console]::IsErrorRedirected) {
        return
    }

    # Profiles can also run in a console process executing a command and then exiting.
    $noExit = $false
    $takesValue = @('configurationfile', 'configurationname', 'custompipename', 'encodedarguments',
        'executionpolicy', 'inputformat', 'outputformat', 'settingsfile', 'windowstyle', 'workingdirectory')
    $skipValue = $false
    foreach ($arg in ([Environment]::GetCommandLineArgs() | Select-Object -Skip 1)) {
        if ($skipValue) { $skipValue = $false; continue }
        if ($arg -notmatch '^[/-].') {
            if (-not $noExit) { return }
            break
        }
        $option = $arg.Substring(1)
        if ($option -like 'non*') { return }
        if ($option -like 'noe*') { $noExit = $true }
        if ($option -in 'ec', 'cwa' -or 'commandwithargs' -like "$option*" -or
            'encodedcommand' -like "$option*" -or 'file' -like "$option*") {
            if (-not $noExit) { return }
            break
        }
        $skipValue = $option -in 'ep', 'if', 'of', 'wd' -or ($takesValue -like "$option*")
    }

    $mgh = Get-Command mgh.exe -CommandType Application -ErrorAction Stop | Select-Object -First 1
    $git = Get-Command git.exe -CommandType Application -ErrorAction Stop | Select-Object -First 1
    $state = @{
        Mgh = $mgh.Path
        Git = $git.Path
        Directory = $ExecutionContext.SessionState.Path.CurrentFileSystemLocation.ProviderPath
        Repo = ''
        Config = $null # MGH_CONFIG
    }

    # The prompt captures PowerShell's success stream. Start the CLI with inherited
    # console handles so progress is visible immediately and its pickers can read keys.
    $run = {
        param([string] $Program, [string] $Arguments, [switch] $Capture)
        $start = New-Object System.Diagnostics.ProcessStartInfo
        $start.FileName = $Program
        $start.Arguments = $Arguments
        $start.WorkingDirectory = $state.Directory
        $start.UseShellExecute = $false
        if ($Capture) {
            $start.RedirectStandardOutput = $true
            $start.RedirectStandardError = $true
            $start.StandardOutputEncoding = [Text.Encoding]::UTF8
        }
        $child = [Diagnostics.Process]::Start($start)
        try {
            if ($Capture) {
                $stderr = $child.StandardError.ReadToEndAsync()
                $output = $child.StandardOutput.ReadToEnd()
            }
            $child.WaitForExit()
            if ($Capture) {
                $null = $stderr.GetAwaiter().GetResult()
                if ($child.ExitCode -eq 0) { $output }
            }
        } finally {
            $child.Dispose()
        }
    }.GetNewClosure()

    $entry = 'internal shell-enter'
    if ($state.Config) {
        $entry = '--config "' + ($state.Config -replace '(\\+)$', '$1$1') + '" ' + $entry
    }
    $repoAtDirectory = {
        $path = & $run $state.Git 'rev-parse --absolute-git-dir' -Capture
        if ($path) { $path.TrimEnd("`r", "`n") }
    }.GetNewClosure()

    $previousPrompt = (Get-Item Function:\prompt).ScriptBlock
    $prompt = {
        $result = & $previousPrompt
        try {
            if ($PWD.Provider.Name -eq 'FileSystem' -and $PWD.ProviderPath -ne $state.Directory) {
                $state.Directory = $PWD.ProviderPath
                $repo = & $repoAtDirectory
                if ($repo -ne $state.Repo) {
                    $state.Repo = $repo
                    & $run $state.Mgh $entry
                }
            }
            & $run $state.Mgh 'internal poll'
        } catch {
            Write-Warning "multigh prompt integration: $_"
        }
        $result
    }.GetNewClosure()

    $env:MGH_SHELL_SESSION = [Guid]::NewGuid().ToString('N')
    $completions = & $run $state.Mgh 'shell completions powershell' -Capture
    if ($completions) { . ([ScriptBlock]::Create($completions)) }
    if (-not (Get-Command git -CommandType Alias,Function,Filter -ErrorAction Ignore)) {
        $shim = Join-Path (Split-Path -Parent $state.Mgh) 'mgh-git.exe'
        if (Test-Path -LiteralPath $shim -PathType Leaf) {
            $env:MGH_POWERSHELL_CONFIG = $state.Config
            Set-Alias -Name git -Value $shim -Scope Global
        } else {
            Write-Warning 'multigh: mgh-git.exe is missing; Git will not wait for background switches.'
        }
    }

    $state.Repo = & $repoAtDirectory
    Set-Item Function:\global:prompt -Value $prompt
    $global:_MghPowerShellLoaded = $true
    & $run $state.Mgh "$entry --startup"
}
