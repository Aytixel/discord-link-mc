# Discord Link MC App
![Logo](https://github.com/Aytixel/discord-link-mc/blob/master/logo.jpg)

Spigot plugin, plus an application, intended to be an equivalent to MumbleLink

# Install Plugin
https://github.com/Aytixel/discord-link-mc-plugin

# Install App
1. Download the last version for your system : https://github.com/Aytixel/discord-link-mc/releases
2. Unzip the file in a folder, all files must remain in the same folder for the application to work
3. You can put the folder containing the app wherever you want on your computer, and create a shortcut
4. You can now launch it, and all you have to do is to follow what is written, until you are given your code/command to link your accounts

**It is possible that after disconnecting or closing Discord Link MC, if you try to restart it you will not hear anyone.
In this case just restart your discord, then restart Discord Link MC, this should solve the problem.**

# Build The App Your Self
1. Clone the repo
2. Create a discord application
3. Create a .env file at the source of project, and set in it this environment variable : ```DISCORD_APPLICATION_ID=<your_discord_application_id>```
4. Follow the installation of the rust discord game sdk crate [here](https://crates.io/crates/discord_game_sdk), all the sdk files are in the sdk folder
5. Now the installation should be done, and the project ready to build
