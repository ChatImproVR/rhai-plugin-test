fn update() {
    if this.x == () {
         this.x = 0;
    }

    this.x += 0.02;

    let k = this.transforms.keys()[0];

    this.transforms[k]["orient"] = quat(this.x, 0., 0.);
}

fn run_me() {
    print("Hello, world!");
    this.x = 0;
    return this;
}

