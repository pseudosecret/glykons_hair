use glicol::Engine;

fn main() {
    let mut engine = Engine::<128>::new();
    let code = "
~trigger: speed 4.0 >> seq 60 _ 60 _ 
~env: ~trigger >> envperc 0.01 0.4
~pitch: ~env >> mul 150 >> add 50
~kick: sin ~pitch >> mul ~env 

~bass_seq: speed 4.0 >> seq _ 40 _ 40
~bass: saw ~bass_seq >> lpf 800 1.0 >> mul 0.3

out: ~kick >> add ~bass >> mul 0.5
    ";
    engine.update_with_code(code);
    let (out, _) = engine.next_block(vec![]);
    println!("Output length: {}", out.len());
    if out.len() > 0 {
        println!("First sample: {:?}", out[0][0]);
    } else {
        println!("Output is EMPTY - Syntax error!");
    }
}
