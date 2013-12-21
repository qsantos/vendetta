#include "game.h"

#include <SFML/Graphics.h>

#include "util.h"
#include "draw.h"

void game_init(game_t* g)
{
	universe_init(&g->u);
	   world_init(&g->w, &g->u);
	graphics_init(&g->g);
}

void game_exit(game_t* g)
{
	graphics_exit(&g->g);
}

void game_loop(game_t* g)
{
	sfTexture* texture = sfTexture_createFromFile("character.png", NULL);
	if (texture == NULL)
		exit(1);

	character_t* player = &g->w.characters[0];

	const sfView* default_view = sfRenderWindow_getDefaultView(g->g.render);
	sfView* world_view = sfView_copy(default_view);

	sfClock* clock = sfClock_create();
	while (sfRenderWindow_isOpen(g->g.render))
	{
		float duration = sfTime_asSeconds(sfClock_getElapsedTime(clock));
		sfClock_restart(clock);

		sfEvent event;
		while (sfRenderWindow_pollEvent(g->g.render, &event))
		{
			if (event.type == sfEvtClosed)
			{
				sfRenderWindow_close(g->g.render);
			}
			else if (event.type == sfEvtKeyPressed)
			{
				if (event.key.code == sfKeyEscape)
					sfRenderWindow_close(g->g.render);
			}
			else if (event.type == sfEvtMouseButtonReleased)
			{
				sfMouseButtonEvent* e = &event.mouseButton;
				if (e->button != sfMouseLeft)
					continue;

				sfVector2i pix = {e->x, e->y};
				sfVector2f pos = sfRenderWindow_mapPixelToCoords(g->g.render, pix, world_view);
				object_t* o = world_objectAt(&g->w, pos.x, pos.y);

				player->go_x = pos.x;
				player->go_y = pos.y;
				player->go_o = o;
			}
		}

		sfBool up    = sfKeyboard_isKeyPressed(sfKeyUp);
		sfBool down  = sfKeyboard_isKeyPressed(sfKeyDown);
		sfBool left  = sfKeyboard_isKeyPressed(sfKeyLeft);
		sfBool right = sfKeyboard_isKeyPressed(sfKeyRight);

		if (up || down || left || right)
		{
			player->go_x = player->o.x + 100 * (right - 2*left);
			player->go_y = player->o.y + 100 * (down  - 2*up);
		}

		world_doRound(&g->w, duration);

		sfRenderWindow_clear(g->g.render, sfBlack);

		sfVector2f pos = {player->o.x, player->o.y};
		sfView_setCenter(world_view, pos);
		sfRenderWindow_setView(g->g.render, world_view);

		draw_world(g->g.render, &g->w);

		sfRenderWindow_setView(g->g.render, default_view);

		sfRenderWindow_display(g->g.render);
	}

	sfClock_destroy(clock);
}
