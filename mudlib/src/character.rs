struct Character<'w, 's, 'p, Target = Untargetted> {
    world: &'w World,
    state: &'s mut WorldState,
    players: &'p mut Players,

    name: String,
    target: Target,
}

struct Untargetted;

struct OtherCharacter {
    name: String,
}

pub fn do_shove(ch: Character, target: OtherCharacter) {

}
