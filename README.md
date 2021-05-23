# Lab Parser

This tool is made to convert animation files used in the game [Tales of Pirates](https://en.wikipedia.org/wiki/Tales_of_Pirates) to a format that is easily consumable by modern 3D animation software such as Blender. The converted file can then be used to create new animations and poses for character models being used in the game.

## Usage

The tool currently offers one option - `lab2dae`. 

This can be used to convert .lab files to .dae (collada) files.

![Usage](static/usage.png)

The collada file can then be imported into Blender in order to look at and modify the bone structure and animations for all character models.

https://user-images.githubusercontent.com/6716458/119255463-952d3f80-bbd9-11eb-9b16-6158e3fb290f.mp4

You can use this option by providing the following arguments to the program -

1st argument: operation type. currently supported - `lab2dae`

2nd argument: file location. currently supported - `.lab` file formats


## On-going work

I'm currently working to support reverse-conversions, so that the updated model can then be converted back to the `.lab` format and be used in the game directly.

I'm also working on ways to add some more data to the collada file, so that we can look at the full textured model within the 3D animation software.

Example: 

This is the same .lab file as the previous video, with mesh and texture information added into the collada file.

https://user-images.githubusercontent.com/6716458/119255449-86df2380-bbd9-11eb-9d0c-8e1ffeec9788.mp4
