#include "game.h"

#include <SFML/Graphics.h>

#include "util.h"
#include "world/draw.h"
#include "overlay/overlay.h"

void game_init(game_t* g)
{
	g->g = graphics_init();
	g->o =  overlay_init();
	g->u = universe_init(g->g);
	g->w =    world_init(g->u);
}

void game_exit(game_t* g)
{
	world_exit   (g->w);
	universe_exit(g->u);
	overlay_exit (g->o);
	graphics_exit(g->g);
}

void game_loop(game_t* g)
{
	g->player = &g->w->characters[0];

	const sfView* default_view = sfRenderWindow_getDefaultView(g->g->render);
	g->g->overlay_view = sfView_copy(default_view);
	g->g->world_view = sfView_copy(default_view);

	sfClock* clock = sfClock_create();
	while (sfRenderWindow_isOpen(g->g->render))
	{
		float duration = sfTime_asSeconds(sfClock_getElapsedTime(clock));
		sfClock_restart(clock);

		sfEvent event;
		while (sfRenderWindow_pollEvent(g->g->render, &event))
		{
			if (event.type == sfEvtClosed)
			{
				sfRenderWindow_close(g->g->render);
			}
			else if (event.type == sfEvtLostFocus)
			{
				g->g->hasFocus = sfFalse;
			}
			else if (event.type == sfEvtGainedFocus)
			{
				g->g->hasFocus = sfTrue;
			}
			else if (event.type == sfEvtResized)
			{
				sfFloatRect rect = {0,0,event.size.width,event.size.height};
				sfView_reset(g->g->overlay_view, rect);
				sfView_reset(g->g->world_view, rect);
			}
			else if (event.type == sfEvtKeyPressed)
			{
				if (event.key.code == sfKeyEscape)
					sfRenderWindow_close(g->g->render);
			}
			else if (event.type == sfEvtKeyReleased)
			{
				sfKeyCode k = event.key.code;
				if (k == sfKeyUp || k == sfKeyDown || k == sfKeyLeft || k == sfKeyRight)
				{
					g->player->go_x = g->player->o.x;
					g->player->go_y = g->player->o.y;
				}
				else if (k == sfKeyB)
				{
					g->o->swbuilding.w.visible ^= 1;
				}
				else if (k == sfKeyI)
				{
					g->o->swinventory.w.visible ^= 1;
				}
				else if (k == sfKeyS)
				{
					g->o->swskills.w.visible ^= 1;
				}
				else if (k == sfKeyE)
				{
					g->o->swequipment.w.visible ^= 1;
				}
				else if (sfKeyF1 <= k && k <= sfKeyF12)
				{
					int id = k - sfKeyF1;
					if (id < g->u->n_mines)
					{
						kindOf_mine_t* t = &g->u->mines[id];
						mine_t* m = world_findMine(g->w, g->player->o.x, g->player->o.y, t);
						if (m != NULL)
							g->player->go_o = (object_t*) m;
					}
				}
			}
			else if (event.type == sfEvtMouseButtonReleased)
			{
				sfMouseButtonEvent* e = &event.mouseButton;
				if (overlay_catch(g->o, g, e->x, e->y, e->button))
					continue;

				if (e->button != sfMouseLeft)
					continue;

				sfVector2i pix = {e->x, e->y};
				sfVector2f pos = sfRenderWindow_mapPixelToCoords(g->g->render, pix, g->g->world_view);
				object_t* o = world_objectAt(g->w, pos.x, pos.y);

				g->player->go_x = pos.x;
				g->player->go_y = pos.y;
				g->player->go_o = o;
			}
		}

		if (g->g->hasFocus)
		{
			sfBool up    = sfKeyboard_isKeyPressed(sfKeyUp);
			sfBool down  = sfKeyboard_isKeyPressed(sfKeyDown);
			sfBool left  = sfKeyboard_isKeyPressed(sfKeyLeft);
			sfBool right = sfKeyboard_isKeyPressed(sfKeyRight);

			if (up || down || left || right)
			{
				g->player->go_x = g->player->o.x + 100 * (right - 2*left);
				g->player->go_y = g->player->o.y + 100 * (down  - 2*up);
				g->player->go_o = NULL;
			}
		}

		world_doRound(g->w, duration);

		sfRenderWindow_clear(g->g->render, sfBlack);

		sfVector2f pos = {g->player->o.x+0.375, g->player->o.y+0.375};
		sfView_setCenter(g->g->world_view, pos);
		sfRenderWindow_setView(g->g->render, g->g->world_view);

		draw_world(g->g, g->w);

		sfRenderWindow_setView(g->g->render, g->g->overlay_view);

		overlay_draw(g->o, g);

		sfRenderWindow_display(g->g->render);
	}

	sfClock_destroy(clock);
}
