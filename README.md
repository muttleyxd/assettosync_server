# AssettoSync Server

Server for AssettoSync - app for downloading and installing mods to Assetto Corsa game.

## Table of contents

* [Usage](#usage)
* [Config file](#config-file)
* [Sceenshots](#screenshots)

## Usage

```
git clone https://github.com/muttleyxd/assettosync_server.git
cd assettosync_server
cp config.json.example config.json
nano config.json #edit config.json according to your needs
cargo run
```

With default config AssettoSync server will be available at `http://localhost:8080/`

Default login data:

login: `admin`

password: `hunter2`

## Config file

You can find example config file in config.json.example
```
{
  "bind_address": "0.0.0.0", # IP address to bind to, 0.0.0.0 means server will bind to every available IP
  "mods": [], # this is used as mod storage database, this will be separated into its own file in the future
  "mod_storage_location": "./mods", # where uploaded mods should be stored
  "port": 8080, # port
  "secret_key": null, # this will be generated when running server for first time
  "server_paths": [ # Assetto Corsa server paths, uploaded mods will be unpacked into content/ directory
    "/home/muttley/servers/assetto_corsa"
  ],
  "users": [ # User list, administrators can manage mods or users (adding, deleting and changing their passwords)
    {
      "is_admin": true,
      "login": "admin",
      "password_hash_sha512": "6b97ed68d14eb3f1aa959ce5d49c7dc612e1eb1dafd73b1e705847483fd6a6c809f2ceb4e8df6ff9984c6298ff0285cace6614bf8daa9f0070101b6c89899e22"
    }
  ]
}
```

## Screenshots

<img src="https://i.imgur.com/u1ZmEeh.png" width="400">

<img src="https://i.imgur.com/QoTLj06.png" width="400">

<img src="https://i.imgur.com/BfAoT4U.png" width="400">

<img src="https://i.imgur.com/AoSjBEO.png" width="400">

<img src="https://i.imgur.com/C5J9lOP.png" width="400">
