-- Aggressive Hunter: sprint toward enemy, fire everything
function think(state, memory)
    if state.enemy then
        return {
            thrust = 1.0,
            turn = state.enemy.lead_turn_to_aim,
            fire_primary = state.enemy.distance < 120,
            fire_secondary = state.enemy.distance < 80
        }
    end
    -- No enemy visible: spiral search
    return { thrust = 0.8, turn = 15 }
end
