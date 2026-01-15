# memes

- allow a user upload an image, gif or video under 50MB with a command
- allow the user to also execute this on a message with a link or uploaded file
- when the command is executed open a modal with a file upload field (if the command wasn't executed on an existing message),
  and let them input into a text field a list of categories separated by spaces (max 10) (should the categories be predetermined? no)
- the user then clicks the upload button to upload the meme to the bot
- the meme gets saved to a folder with the sanitized name of the first category
- the user can later on then run a command /meme [category] [ordered?] to upload a meme from that category
- the ordered flag would send memes from that category from oldest to newest for that server, default is false
