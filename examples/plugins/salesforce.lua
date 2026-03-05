-- Salesforce CLI integration plugin for repoman
-- Automates common Salesforce DX tasks when working with SFDX projects.
--
-- What it does:
--   post_clone:  Detects sfdx-project.json, authenticates a scratch org,
--                pushes source, and runs default Apex tests.
--   post_sync:   Re-pushes source to the scratch org after upstream changes.
--   pre_destroy:  Deletes the scratch org before the clone is removed.
--
-- Requirements:
--   - Salesforce CLI (`sf` or `sfdx`) installed and on PATH
--   - A Dev Hub already authenticated (`sf org login web --set-default-dev-hub`)
--   - sfdx-project.json in the repo root
--
-- Configuration:
--   Set these environment variables to customize behavior:
--     REPOMAN_SF_SCRATCH_DEF   Path to scratch org definition (default: config/project-scratch-def.json)
--     REPOMAN_SF_ALIAS_PREFIX  Prefix for scratch org aliases (default: "repoman")
--     REPOMAN_SF_DURATION      Scratch org duration in days (default: 7)
--     REPOMAN_SF_SKIP_TESTS    Set to "1" to skip running tests on clone

local function is_sfdx_project(path)
    local f = io.open(path .. "/sfdx-project.json", "r")
    if f then
        f:close()
        return true
    end
    return false
end

local function sf_alias(ctx)
    local prefix = os.getenv("REPOMAN_SF_ALIAS_PREFIX") or "repoman"
    -- Sanitize clone name for use as an org alias
    return prefix .. "-" .. ctx.clone_name:gsub("[^%w%-]", "-")
end

-- After cloning: create a scratch org, push source, run tests
repoman.on("post_clone", function(ctx)
    if not is_sfdx_project(ctx.clone_path) then
        return
    end

    local alias = sf_alias(ctx)
    local scratch_def = os.getenv("REPOMAN_SF_SCRATCH_DEF") or "config/project-scratch-def.json"
    local duration = os.getenv("REPOMAN_SF_DURATION") or "7"

    repoman.log("info", "Salesforce project detected — setting up scratch org '" .. alias .. "'")

    -- Create scratch org
    local cmd = "cd " .. ctx.clone_path
        .. " && sf org create scratch"
        .. " --definition-file " .. scratch_def
        .. " --alias " .. alias
        .. " --duration-days " .. duration
        .. " --set-default"
        .. " --json"
    local result = repoman.exec(cmd)
    if not result or result == "" then
        repoman.log("warn", "Failed to create scratch org for " .. ctx.clone_name)
        return
    end
    repoman.log("info", "Scratch org '" .. alias .. "' created")

    -- Push source to scratch org
    repoman.log("info", "Pushing source to scratch org...")
    repoman.exec("cd " .. ctx.clone_path .. " && sf project deploy start --target-org " .. alias)

    -- Run default Apex tests (unless skipped)
    local skip_tests = os.getenv("REPOMAN_SF_SKIP_TESTS")
    if skip_tests ~= "1" then
        repoman.log("info", "Running Apex tests...")
        repoman.exec("cd " .. ctx.clone_path
            .. " && sf apex run test --target-org " .. alias
            .. " --test-level RunLocalTests --wait 10")
    end

    repoman.log("info", "Salesforce setup complete for " .. ctx.clone_name)
end)

-- After syncing: re-deploy source to the scratch org
repoman.on("post_sync", function(ctx)
    if not is_sfdx_project(ctx.clone_path) then
        return
    end

    local alias = sf_alias(ctx)

    -- Check if the scratch org still exists
    local check = repoman.exec("sf org display --target-org " .. alias .. " --json 2>/dev/null")
    if not check or check == "" then
        repoman.log("info", "No scratch org found for " .. ctx.clone_name .. ", skipping deploy")
        return
    end

    repoman.log("info", "Re-deploying source to scratch org '" .. alias .. "' after sync")
    repoman.exec("cd " .. ctx.clone_path .. " && sf project deploy start --target-org " .. alias)
end)

-- Before destroying: delete the scratch org
repoman.on("pre_destroy", function(ctx)
    if not is_sfdx_project(ctx.clone_path) then
        return
    end

    local alias = sf_alias(ctx)

    repoman.log("info", "Deleting scratch org '" .. alias .. "' before destroy")
    repoman.exec("sf org delete scratch --target-org " .. alias .. " --no-prompt 2>/dev/null || true")
end)
