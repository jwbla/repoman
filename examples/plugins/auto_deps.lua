-- Detects package manager and installs dependencies post-clone
repoman.on("post_clone", function(ctx)
    local path = ctx.clone_path
    local checks = {
        { file = "package.json", cmd = "npm ci" },
        { file = "Cargo.toml",  cmd = "cargo fetch" },
        { file = "go.mod",      cmd = "go mod download" },
        { file = "requirements.txt", cmd = "pip install -r requirements.txt" },
        { file = "Gemfile",     cmd = "bundle install" },
        { file = "pyproject.toml", cmd = "pip install -e ." },
    }
    for _, check in ipairs(checks) do
        local f = io.open(path .. "/" .. check.file, "r")
        if f then
            f:close()
            repoman.log("info", "Detected " .. check.file .. ", running: " .. check.cmd)
            repoman.exec("cd " .. path .. " && " .. check.cmd)
            return
        end
    end
end)
