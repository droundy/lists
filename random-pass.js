function random_pass() {
    const validWords = ['Dog', 'Cat', 'Worm', 'Duck','Horse','Mule','Donkey',
                        'Giraffe', 'Colt', 'Mare', 'Puppy', 'Pup', 'Kitty',
                        'Kitten', 'Sheep', 'Lamb', 'Goat', 'Elephant', 'Rhino',
                        'Lion', 'Cub', 'Wolf', 'Fox', 'Bird', 'Crow', 'Raven',
                        'Robin',

                        'Song', 'Piece',

                        'Red', 'Orange', 'Pink', 'Yellow', 'Green', 'Cyan', 'Blue',
                        'Magenta', 'Purple', 'White', 'Black',

                        'Little', 'Small', 'Big', 'Tiny', 'Huge', 'Furry',

                        'Eat', 'Tap', 'Kiss', 'Push', 'Poop', 'Pee', 'Hug',
                        'Throw', 'Toss', 'Write', 'Read', 'Compute', 'Fart',
                        'Screw', 'Drink', 'Feed', 'Play', 'Sing',
                       ];
    let array = new Uint32Array(4);
    window.crypto.getRandomValues(array);
    var pass = '';
    for (var i in array) {
        pass += validWords[array[i] % validWords.length];
    }
    return pass;
}
