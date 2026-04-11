mod http_if;

#[tokio::main]
async fn main() {
    // bond_lib::bond::default::default(None);
    let (agent,) = bond_lib::bond::default::default(None).await;
    let agent_ctl = agent.new_ctl();

    let join_a = tokio::spawn(agent.run());
    let join_b = tokio::spawn(http_if::ws::run(agent_ctl));

    tokio::signal::ctrl_c().await.unwrap();


    // agent_ctl.send_message("main","RUN ls in shell").await;
    // agent_ctl.send_message("main","Hello").await;
    // agent_ctl.send_message("main","World").await;
    // agent_ctl.send_message("main","::))").await;
    // loop {
    //     println!("{:?}", agent_ctl.recv().await);
    // }

    // join.await.unwrap();
}