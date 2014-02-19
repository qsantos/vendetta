/*\
 *  Role playing, management and strategy game
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
#include "world/world_save.h"
#include "world/world_gen.h"
#include "world/draw.h"
#include "overlay/overlay.h"

void game_init(game_t* g, settings_t* s, graphics_t* gr, char load)
{
	g->s = s;
	g->g = gr;

	g->o = CALLOC( overlay_t, 1);
	g->u = CALLOC(universe_t, 1);
	g->w = CALLOC(   world_t, 1);

	 overlay_init(g->o, g);
	universe_init(g->u, g);
	   world_init(g->w, g);

	if (load)
	{
		const char* filename = "game.save";
		FILE* f = fopen(filename, "r");
		if (f == NULL)
		{
			fprintf(stderr, "Could not load game\n");
			exit(1);
		}
		game_load(g, f);
		fclose(f);
	}
	else
	{
		g->w->rows = s->map_height;
		g->w->cols = s->map_width;
		world_genmap(g->w, s->seed);
		world_start(g->w);

		for (size_t i = 0; i < N_STATUSES; i++)
			g->autoEat[i] = 0;
	}

	sfVector2u size = sfRenderWindow_getSize(g->g->render);
	sfFloatRect rect = {0,0,size.x,size.y};
	g->g->overlay_view = sfView_createFromRect(rect);
	g->g->world_view   = sfView_createFromRect(rect);

	g->player = NULL;
	pool_t* p = &g->w->objects;
	for (size_t i = 0; i < p->n_objects; i++)
	{
		character_t* c = character_get(&g->w->objects, i);
		if (c == NULL)
			continue;

		if (g->player == NULL)
			g->player = c;
		else if (!load)
			c->ai = &g->u->bots[rand() % g->u->n_bots];
	}
	if (g->player == NULL)
	{
		fprintf(stderr, "No character\n");
		exit(1);
	}

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
			m[i] = character_maxOfMaterial(g->player, t);
		}
	}
}

void game_exit(game_t* g)
{
	sfView_destroy(g->g->overlay_view);
	sfView_destroy(g->g->world_view);

	   world_exit(g->w);
	universe_exit(g->u);
	 overlay_exit(g->o);

	free(g->w);
	free(g->u);
	free(g->o);
}

void game_loop(game_t* g)
{
	char hasFocus = 1;
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
				hasFocus = 0;
			}
			else if (event.type == sfEvtGainedFocus)
			{
				hasFocus = 1;
			}
			else if (event.type == sfEvtResized)
			{
				sfFloatRect rect = {0,0,event.size.width,event.size.height};
				sfView_reset(g->g->overlay_view, rect);
				sfView_reset(g->g->world_view,   rect);
				sfView_zoom (g->g->world_view,   zoom);
			}

			if (!hasFocus)
				continue;

			if (event.type == sfEvtKeyReleased)
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
					building_t* b = building_get(&g->w->objects, g->player->hasBuilding);
					if (b != NULL)
						g->player->go_o = b->o.uuid;
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
						g->player->go_o = t->o.uuid;
				}
				else if (k == sfKeyW)
				{
					building_t* t = world_findEnnemyBuilding(g->w, g->player);
					if (t != NULL)
						g->player->go_o = t->o.uuid;
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
							g->player->go_o = m->o.uuid;
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
				if (o == NULL)
					g->player->go_o = -1;
				else
					g->player->go_o = o->uuid;
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

		if (hasFocus)
		{
			sfBool up    = sfKeyboard_isKeyPressed(sfKeyUp);
			sfBool down  = sfKeyboard_isKeyPressed(sfKeyDown);
			sfBool left  = sfKeyboard_isKeyPressed(sfKeyLeft);
			sfBool right = sfKeyboard_isKeyPressed(sfKeyRight);

			if (up || down || left || right)
			{
				g->player->go_x = g->player->o.x + 100 * (right - 2*left);
				g->player->go_y = g->player->o.y + 100 * (down  - 2*up);
				g->player->go_o = -1;
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
				g->player->go_o = -1;
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
		character_t* c = g->player;
		for (size_t i = 0; i < N_STATUSES; i++)
		{
			float max = character_maxOfStatus(c, i);
			float p = c->statuses[i] / max;
			if (g->autoEat[i] && p < 0.5)
				character_eatFor(g->player, i);
		}

		// do round
		world_doRound(g->w, duration);
	}

	sfClock_destroy(maintain);
	sfClock_destroy(clock);
}

void game_save(game_t* g, FILE* f)
{
	for (size_t i = 0; i < N_STATUSES; i++)
		fprintf(f, "%i/", g->autoEat[i]);
	fprintf(f, "\n");
	world_save(g->w, f);
}

#define CLINE(...) do { \
	if (fscanf(f, __VA_ARGS__) < 0){ \
		fprintf(stderr, "Missing line in save\n"); \
		exit(1); \
	} \
	} while (0);
void game_load(game_t* g, FILE* f)
{
	for (size_t i = 0; i < N_STATUSES; i++)
	{
		int v;
		CLINE("%i/", &v);
		g->autoEat[i] = v;
	}
	CLINE("\n");
	world_load(g->w, f);
}
