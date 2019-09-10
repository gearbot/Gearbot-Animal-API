# Gearbot Animal API
[![Build Status](https://dev.azure.com/GearBot/GearBot/_apis/build/status/gearbot.Gearbot-Animal-API?branchName=master)](https://dev.azure.com/GearBot/GearBot/_build/latest?definitionId=1&branchName=master)

## About
This is the API that [Gearbot](https://github.com/gearbot/Gearbot) uses for retrieving animal facts. It does exactly what it
sounds like.

## Setup
1. After downloading the repository and generating the desired binary, copy `default_config.toml` to `config.toml`.
2. Next, edit all the values needed inside the config. If you want to use the admin interface, make sure to add some admins.
See `default_config.toml` for more information.

3. Make sure the defined `facts_dir` has either or / both `cat_facts.json` and `dog_facts.json`.
4. Start the app

## Usage
Consumption:
To get a fact, visit `/{animal}/fact/`. Currently, only dog and cat facts are supported.
Facts are returned in the format of:
```json
{
    "id": 15889153292243741260,
    "content": "Some cool fact"
}
```

Admin Interface:
An easy to use admin API is provided under `/admin`. Admins are defined in the `config.toml` file. When no admins exist
in the configuration, any requests to the admin endpoints will fail due to invalid auth.

An admin can possess any combination of delete, add, or no permissions:
```toml
[[admins]]
name = "SpookyAdmin"
key = "SuperSecretKey"
[admins.permissions]
add = true
delete = false
```

To add a new fact to the system, send a `POST` request to `/admin/add`:
```json
{
    "animal_type": "Cat",
    "content": "Huzzah, a new fact!",
    "auth": "SuperSecretKey"
}
```

Delete works in a similar fashion, but with a request to `/admin/delete`:
```json
{
    "animal_type": "Dog",
    "fact_id": 82872012121262,
    "auth": "SuperSecretKey"
}
```
Note: For the above, the animal type must be capitlized. Ex: `Cat` works, but `cat` does not.

If the admin request was malformed somehow, an error will be returned in the format of:
```json
{
    "code": 401,
    "message": "Bad authorization!"
}
```

## Build Steps:
1. Make sure Rust is installed on your system with the appropriate toolchains
2. Clone this repository to a folder somewhere
3. `cd` into the created directory
4. Run `cargo build` to produce a binary 
5. Find some facts and put them in the `facts` directory as `cat_facts.json` and `dog_facts.json` respectively
6. Admin keys are loaded from `admin_keys.json`. See `example_keys.json` for the required structure.

## Testing
The entire API is extensively tested, covering all possible *valid* behavior. Bad syntax, variable types, etc are automatically handled by Actix.
When contributing, we ask that you assure that existing tests pass, or are modified to fit changed behavior if required. It is also preferable that
any new endpoints added have corresponding integration tests.

Testing is fairly straight forward: `cargo test`. See Rust's documentation for more information and specifics on using it.