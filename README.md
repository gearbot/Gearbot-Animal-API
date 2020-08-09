# Gearbot Animal API
[![Build Status](https://dev.azure.com/GearBot/GearBot/_apis/build/status/gearbot.Gearbot-Animal-API?branchName=master)](https://dev.azure.com/GearBot/GearBot/_build/latest?definitionId=1&branchName=master)

## About
This is the API that [Gearbot](https://github.com/gearbot/Gearbot) uses for retrieving animal facts. It does exactly what it
sounds like.

## Setup
1. After downloading the repository and generating the desired binary, copy `default_config.toml` to `config.toml`.
2. Next, edit all the values needed inside the config. If you want to use the admin interface, make sure to add some admins.
See `default_config.toml` for more information.

3. Make sure the defined `facts_dir` has either/or `cat_facts.json` and `dog_facts.json`.
4. Start the app

## Usage
### Consumption:

#### Facts:
To get a fact, visit `/{animal}/fact/`. Currently, only dog and cat facts are supported.
Facts are returned in the format of:
```json
{
    "id": 15889153292243741260,
    "content": "Some cool fact"
}
```

#### Flags
The AnimalAPI has optional support for submitting fact "flags", or notices. This feature is meant to
be used behind another service, such as a dashboard, because the API takes no responsibility managing who flags are sent by. All it sees is a authorization key. It is up to the implementor to rate limit, allow users, etc.

In order to enable this feature, two things must be done first. First, set `flagging_enabled` to `true` in the configuration. Second, create a flagger location inside the config as well. See the default config for an example.

In order to set a flag attempt, a request must be POSTed to the `/flag` endpoint:
```json
{
    "fact_type": "Dog",
    "fact_id": 29230202030113,
    "reason": "Why I Don't Like It",
    "key": "SecretKey",
    "flagger": "SomeLocation"
}
```

The `key` field is required to be populated with a key defined inside a flagger location in the config.

The `reason` and `flagger` fields are both optional. If the `flagger` field is not set, then the entry's submitter field will default to the name associated with the authorization key used. This is so implementors have the ability to make an end user the flagger, if they so choose. So instead of `Dashboard` being the flagger, it could be `Sam12345`.


### Admin Interface:
An easy to use admin API is provided under `/admin`. Admins are defined in the `config.toml` file. When no admins exist
in the configuration, any requests to the admin endpoints will fail due to invalid authorization.

An admin can possess any combination of add, delete, view, or no permissions:
```toml
[[admins]]
name = "SpookyAdmin"
key = "SuperSecretKey"
[admins.permissions]
add_fact = true
delete_fact = false
view_facts = true
...
```
#### Facts
To add a new fact to the system, send a `POST` request to `/admin/fact/add`:
```json
{
    "animal_type": "Cat",
    "content": "Huzzah, a new fact!",
    "key": "SuperSecretKey"
}
```

To delete a fact from the system, send a `POST` request to `/admin/fact/delete`:
```json
{
    "animal_type": "Dog",
    "fact_id": 82872012121262,
    "key": "SuperSecretKey"
}
```

To see all the current facts loaded for a specific animal at once, send a `POST` request to `/admin/fact/list`:
```json
{
    "animal_type": "Dog",
    "key": "SuperSecretKey"
}
```

#### Flags
The admin interface also provides a way for users to add, delete, or view the current fact flags in a similar fashion to handling facts. The required permissions can be found in the example config.

To add a flag, send a `POST` request to `/admin/flag/add`:
```json
 {
    "key": "SuperSecretKey",
    "fact_id": 6682463169732688062,
    "reason": "It was weird",
    "fact_type": "Dog"
}
```
To delete a flag, send a `POST` request to `/admin/flag/delete`:
```json
{
    "key": "SuperSecretKey",
    "flag_id": 6682463169732628062
}
```

To view all the current flags, send a `POST` request to `/admin/flag/list` with just a key:
```json
{
   "key": "SuperSecretKey"
}
```

Note: For all of the above, the animal type must be capitalized. Ex: `Cat` works, but `cat` does not.

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