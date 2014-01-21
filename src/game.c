/*\
 *  Role stayhere, management and strategy game
 *  Copyright (C) 2013-2014 Quentin SANTOS
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program.  If not, see <http://www.gnu.org/licenses/>.
\*/

#include "game.h"

#include <stdlib.h>
#include <math.h>
#include <SFML/Graphics.h>

#include "mem.h"
#include "file.h"
#include "world/draw.h"
#include "overlay/overlay.h"

void game_init(game_t* g, settings_t* s, graphics_t* gr)
{
	g->s = s;
	g->g = gr;

	g->o =  overlay_init(g);
	g->u = universe_init(g);
	g->w =    world_init(g);

	g->player = &g->w->characters[0];

	for (size_t i = 0; i < g->w->n_characters; i++)
	{
		character_t* c = &g->w->characters[i];
		if (c == g->player)
			continue;
		c->ai = &g->u->bots[rand() % g->u->n_bots];
	}

	for (size_t i = 0; i < N_STATUSES; i++)
		g->autoEat[i] = 0;

	if (s->godmode)
	{
		skill_t* s = g->player->skills;
		for (size_t i = 0; i < g->u->n_skills; i++)
			s[i] = 1000;
		s[SK_WALK] = 100;
	}
	if (s->quickstart)
	{
		float* m = g->player->inventory.materials;
		for (size_t i = 0; i < g->u->n_materials; i++)
		{
			kindOf_material_t* t = &g->u->materials[i];
			m[i] = character_maxOf(g->player, t);
		}
	}

	sfVector2u size = sfRenderWindow_getSize(g->g->render);
	sfFloatRect rect = {0,0,size.x,size.y};
	g->g->overlay_view = sfView_createFromRect(rect);
	g->g->world_view   = sfView_createFromRect(rect);
}

void game_exit(game_t* g)
{
	sfView_destroy(g->g->overlay_view);
	sfView_destroy(g->g->world_view);

	   world_exit(g->w);
	universe_exit(g->u);
	 overlay_exit(g->o);
}

void game_loop(game_t* g)
{
	float zoom = 1;
	sfClock* clock = sfClock_create();
	sfClock* maintain = sfClock_create();
	float fpssum = 0;
	int fpscount = 0;
	float fpslast = 0;
	char stayhere = 1;
	while (stayhere && sfRenderWindow_isOpen(g->g->render))
	{
		// handle user input
		sfEvent event;
		while (stayhere && sfRenderWindow_pollEvent(g->g->render, &event))
		{
			if (event.type == sfEvtClosed)
			{
				stayhere = 0;
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
				sfView_reset(g->g->world_view,   rect);
				sfView_zoom (g->g->world_view,   zoom);
			}
			else if (event.type == sfEvtKeyReleased)
			{
				sfKeyCode k = event.key.code;
				if (k == sfKeyEscape)
					stayhere = 0;
				else if (k == sfKeyUp || k == sfKeyDown || k == sfKeyLeft || k == sfKeyRight)
				{
					g->player->go_x = g->player->o.x;
					g->player->go_y = g->player->o.y;
				}
				else if (k == sfKeyReturn)
				{
					if (g->player->hasBuilding != NULL)
						g->player->go_o = &g->player->hasBuilding->o;
				}
				else if (k == sfKeyDelete)
				{
					character_delHome(g->player);
				}
				else if (k == sfKeyB)
				{
					g->o->swbuilding.w.visible ^= 1;
				}
				else if (k == sfKeyO)
				{
					g->o->switems.w.visible ^= 1;
				}
				else if (k == sfKeyM)
				{
					g->o->swmaterials.w.visible ^= 1;
				}
				else if (k == sfKeyC)
				{
					g->o->swskills.w.visible ^= 1;
				}
				else if (k == sfKeyE)
				{
					g->o->swequipment.w.visible ^= 1;
				}
				else if (k == sfKeyX)
				{
					character_t* t = world_findEnnemyCharacter(g->w, g->player);
					if (t != NULL)
						g->player->go_o = &t->o;
				}
				else if (k == sfKeyW)
				{
					building_t* t = world_findEnnemyBuilding(g->w, g->player);
					if (t != NULL)
						g->player->go_o = &t->o;
				}
				else if (k == sfKeySpace)
				{
					g->o->swbuilding .w.visible ^= 1;
					g->o->switems    .w.visible ^= 1;
					g->o->swmaterials.w.visible ^= 1;
					g->o->swskills   .w.visible ^= 1;
					g->o->swequipment.w.visible ^= 1;
				}
				else if (sfKeyF1 <= k && k <= sfKeyF12)
				{
					size_t id = k - sfKeyF1;
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
				if (overlay_catch(g, e->button))
					continue;

				if (e->button != sfMouseLeft)
					continue;

				sfVector2i pix = {e->x, e->y};
				sfVector2f pos = sfRenderWindow_mapPixelToCoords(g->g->render, pix, g->g->world_view);
				object_t* o = world_objectAt(g->w, pos.x, pos.y, &g->player->o);

				g->player->go_x = pos.x;
				g->player->go_y = pos.y;
				g->player->go_o = o;
			}
			else if (event.type == sfEvtMouseButtonPressed)
			{
				sfClock_restart(maintain);
				overlay_catch(g, -event.mouseButton.button-1);
			}
			else if (event.type == sfEvtMouseMoved)
			{
				overlay_move(g);
			}
			else if (event.type == sfEvtMouseWheelMoved)
			{
				int delta = event.mouseWheel.delta;
				if (sfKeyboard_isKeyPressed(sfKeyLControl))
				{
					float dzoom = pow(1.1, -delta);
					sfView_zoom(g->g->world_view, dzoom);
					zoom *= dzoom;
				}
				else
					overlay_wheel(g, delta);
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

			if (sfMouse_isButtonPressed(sfMouseLeft) &&
			(sfKeyboard_isKeyPressed(sfKeyLShift) ||
			sfTime_asSeconds(sfClock_getElapsedTime(maintain)) > 0.5
			))
			{
				sfVector2i pix = sfMouse_getPosition((sfWindow*) g->g->render);
				sfVector2f pos = sfRenderWindow_mapPixelToCoords(g->g->render, pix, g->g->world_view);
				g->player->go_x = pos.x;
				g->player->go_y = pos.y;
				g->player->go_o = NULL;
			}
		}

		// update display
		sfRenderWindow_clear(g->g->render, sfBlack);

		sfVector2f pos = {g->player->o.x, g->player->o.y};
		sfVector2f size = sfView_getSize(g->g->world_view);
		pos.x = fmax(pos.x, size.x/2-g->w->o.w/2);
		pos.y = fmax(pos.y, size.y/2-g->w->o.h/2);
		pos.x = fmin(pos.x,-size.x/2+g->w->o.w/2);
		pos.y = fmin(pos.y,-size.y/2+g->w->o.h/2);
		sfView_setCenter(g->g->world_view, pos);

		sfRenderWindow_setView(g->g->render, g->g->world_view);
		draw_world(g->g, g->player, g->w);
		sfRenderWindow_setView(g->g->render, g->g->overlay_view);

		overlay_draw(g, 1);
		int t = overlay_cursor(g);
		graphics_drawCursor(g->g, t);
		sfRenderWindow_display(g->g->render);

		// check frame duration
		float duration = sfTime_asSeconds(sfClock_restart(clock));
		g->g->step += duration;

		// measure FPS
		fpssum += 1. / duration;
		fpscount++;
		fpslast += duration;
		if (fpslast >= 1.)
		{
			g->g->fps = fpssum / fpscount;
			fpssum = 0;
			fpscount = 0;
			fpslast = 0;
		}

		// check player statuses
		for (size_t i = 0; i < N_STATUSES; i++)
			if (g->autoEat[i] && g->player->statuses[i] < 10)
				character_eatFor(g->player, i);

		// do round
		world_doRound(g->w, duration);
	}

	sfClock_destroy(maintain);
	sfClock_destroy(clock);
}
