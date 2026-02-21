-- Defensive Kiter: maintain distance, orbit and fire, evade when close
function think(state, memory)
    memory.turns_since_seen = (memory.turns_since_seen or 0) + 1

    if state.enemy then
        memory.turns_since_seen = 0
        memory.last_enemy_pos = state.enemy.position

        if state.enemy.distance < 50 then
            -- Too close: evade
            return {
                thrust = 1.0,
                turn = state.enemy.turn_to_aim + 180,
                shield = state.self_ship.energy > 30
            }
        elseif state.enemy.distance < 100 then
            -- Ideal range: orbit and fire
            return {
                thrust = 0.6,
                turn = state.enemy.lead_turn_to_aim + 12,
                fire_primary = true,
                fire_secondary = state.self_ship.energy > 40
            }
        else
            -- Close the gap cautiously
            return { thrust = 0.7, turn = state.enemy.lead_turn_to_aim }
        end
    end

    -- Search pattern
    return { thrust = 0.6, turn = 20 }
end
