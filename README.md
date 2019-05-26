# Gearbot Animal API
[![Build Status](https://dev.azure.com/GearBot/GearBot/_apis/build/status/gearbot.Gearbot-Animal-API?branchName=master)](https://dev.azure.com/GearBot/GearBot/_build/latest?definitionId=1&branchName=master)

This is the API that [Gearbot](https://github.com/AEnterprise/Gearbot) uses for retrieving animal facts.

## Build Steps:
1. Make sure Rust is installed on your system with the appropriate toolchains
2. Clone this repository to a folder somewhere
3. `cd` into the created directory
4. Run `cargo build` to produce a binary 
5. Find some facts and put them in `cat_facts.json` and `dog_facts.json`!
6. Admin keys are loaded from `admin_keys.json`. See `example_keys.json` for the required structure.
