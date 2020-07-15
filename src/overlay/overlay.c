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

#include "overlay.h"

#include <stdlib.h>
#include <stdio.h>
#include <math.h>

#include "../mem.h"
#include "../widgets.h"
#include "../string.h"

#define CATCH_NONE  (-1)
#define CATCH_BUTTON (0)
#define CATCH_ORDER  (1)
#define CATCH_STATUS (2)
#define CATCH_NTYPES (3)
#define CATCH_CODE(TYPE, IDX) (CATCH_NTYPES*(IDX)+TYPE)
#define CATCH_TYPE(CODE) ((CODE)%3)
#define CATCH_IDX( CODE) ((CODE)/3)

void overlay_init(overlay_t* o, game_t* g)
{
	   ov_build_init(&o->build);
	 swbuilding_init(&o->swbuilding,  g);
	    switems_init(&o->switems,     g);
	swmaterials_init(&o->swmaterials, g);
	   swskills_init(&o->swskills,    g);
	swequipment_init(&o->swequipment, g);

	o->n_subwindows = 5;
	o->sw = CALLOC(subwindow_t*, o->n_subwindows);
	o->sw[0] = &o->swbuilding .w;
	o->sw[1] = &o->swmaterials.w;
	o->sw[2] = &o->swequipment.w;
	o->sw[3] = &o->swskills   .w;
	o->sw[4] = &o->switems    .w;

	o->selected = -1;

	o->msg = NULL;
	o->msgDelay = 0;
}

void overlay_exit(overlay_t* o)
{
	free(o->msg);

	free(o->sw);

	swequipment_exit(&o->swequipment);
	   swskills_exit(&o->swskills);
	    switems_exit(&o->switems);
	swmaterials_exit(&o->swmaterials);
	 swbuilding_exit(&o->swbuilding);
	   ov_build_exit(&o->build);
}

static int overlay_buttons(game_t* g, char do_draw)
{
	static sfSprite* sprite = NULL;
	if (sprite == NULL)
		sprite = assets_sprite(g->a, "data/buttons.png");
	for (size_t i = 0; i < g->o->n_subwindows; i++)
	{
		subwindow_t* w = g->o->sw[i];

		sfVector2u size = sfRenderWindow_getSize(g->g->render);
		sfSprite_setPosition(sprite, (sfVector2f){size.x - 200 + 24*i, 5});

		sfColor color = w->visible ? (sfColor){255,255,255,127} : sfWhite;
		sfSprite_setColor(sprite, color);

		sfIntRect rect = {24*i, 0, 24, 24};
		sfSprite_setTextureRect(sprite, rect);
		if (do_draw)
			sfRenderWindow_drawSprite(g->g->render, sprite, NULL);
		else if (sfSprite_contains(sprite, g->o->mouse))
			return i;
	}

	size_t i = 7;
	sfVector2u size = sfRenderWindow_getSize(g->g->render);
	sfSprite_setPosition(sprite, (sfVector2f){size.x - 200 + 24*g->o->n_subwindows, 5});
	sfSprite_setColor(sprite, sfWhite);
	sfIntRect rect = {24*i, 0, 24, 24};
	sfSprite_setTextureRect(sprite, rect);
	if (do_draw)
		sfRenderWindow_drawSprite(g->g->render, sprite, NULL);
	else if (sfSprite_contains(sprite, g->o->mouse))
		return i;
	return CATCH_NONE;
}
static int overlay_orders(game_t* g, char do_draw)
{
	static sfSprite* sprite = NULL;
	if (sprite == NULL)
		sprite = assets_sprite(g->a, "data/orders.png");

	float x = 100;
	float y = 0;
	for (size_t i = 1; i < 12; i++)
	{
		sfVector2f pos = {x, y};
		sfSprite_setPosition(sprite, pos);

		sfIntRect rect = {28*i, 0, 28, 28};
		sfSprite_setTextureRect(sprite, rect);

		if (do_draw)
			sfRenderWindow_drawSprite(g->g->render, sprite, NULL);
		else if (sfSprite_contains(sprite, g->o->mouse))
			return i;

		x += 28;
	}
	return CATCH_NONE;
}
static int overlay_statuses(game_t* g, char do_draw)
{
	character_t* c = g->player;

	static sfText* text = NULL;
	if (text == NULL)
	{
		text = sfText_create();
		sfColor col = {255, 255, 255, 255};
		sfText_setColor(text, col);
		sfText_setFont(text, g->a->font);
		sfText_setCharacterSize(text, 15);
	}

	sfVector2u size = sfRenderWindow_getSize(g->g->render);
	float x = 10;
	float y = size.y - 10 - 30 * N_STATUSES;
	for (int i = 0; i < N_STATUSES; i++)
	{
		if (!do_draw &&
		    x <= g->o->mouse.x && g->o->mouse.x <= x + 150 &&
		    y <= g->o->mouse.y && g->o->mouse.y <= y + 20)
			return i;

		float max = character_maxOfStatus(c, i);
		float p = c->statuses[i] / max;
		if (do_draw)
			draw_progressbar(g->g, x, y, 150, 20, p, g->autoEat[i]);

		sfText_setUTF8(text, g->u->statuses[i].name); // TODO
		sfVector2f pos = {x+5, y};
		sfText_setPosition(text, pos);
		if (do_draw)
			sfRenderWindow_drawText(g->g->render, text, NULL);

		char buffer[1024];
		snprintf(buffer, 1024, "%.0f/%.0f", floor(c->statuses[i]), floor(max));
		sfText_setUTF8(text, buffer);
		sfFloatRect rect = sfText_getLocalBounds(text);
		pos.x = x + 140 - rect.width;
		sfText_setPosition(text, pos);
		if (do_draw)
			sfRenderWindow_drawText(g->g->render, text, NULL);

		y += 30;
	}
	return CATCH_NONE;
}
int overlay_draw(game_t* g, char do_draw)
{
	g->o->mouse = overlay_mouse(g);

	int i = overlay_buttons(g, do_draw);
	if (i != CATCH_NONE)
		return CATCH_CODE(CATCH_BUTTON, i);

	i = overlay_orders(g, do_draw);
	if (i != CATCH_NONE)
		return CATCH_CODE(CATCH_ORDER, i);

	i = overlay_statuses(g, do_draw);
	if (i != CATCH_NONE)
		return CATCH_CODE(CATCH_STATUS, i);

	if (!do_draw)
		return CATCH_NONE;

	   ov_build_draw(&g->o->build,       g, 1);
	swequipment_draw(&g->o->swequipment, g, 1);
	   swskills_draw(&g->o->swskills,    g, 1);
	swmaterials_draw(&g->o->swmaterials, g, 1);
	    switems_draw(&g->o->switems,     g, 1);
	 swbuilding_draw(&g->o->swbuilding,  g, 1);

	static sfText* text = NULL;
	if (text == NULL)
	{
		text = sfText_create();
		sfColor col = {255, 255, 255, 255};
		sfText_setColor(text, col);
		sfText_setFont(text, g->a->font);
		sfText_setCharacterSize(text, 15);
	}

	sfVector2u size = sfRenderWindow_getSize(g->g->render);

	char buffer[1024];
	character_currentAction(g->player, buffer, 1024);
	sfText_setUTF8(text, buffer);
	sfText_setPosition(text, (sfVector2f){180, size.y - 30});
	sfRenderWindow_drawText(g->g->render, text, NULL);

	snprintf(buffer, 1024, "Or : %.0f", floor(g->player->inventory.money));
	sfText_setUTF8(text, buffer);
	sfText_setPosition(text, (sfVector2f){size.x/2, size.y - 30});
	sfRenderWindow_drawText(g->g->render, text, NULL);

	snprintf(buffer, 1024, "FPS : %.0f", floor(g->fps));
	sfText_setUTF8(text, buffer);
	sfText_setPosition(text, (sfVector2f){size.x - 80, size.y - 30});
	sfRenderWindow_drawText(g->g->render, text, NULL);

	if (g->o->msgDelay != 0)
	{
		snprintf(buffer, 1024, "%s", g->o->msg);
		sfText_setUTF8(text, buffer);
		sfFloatRect rect = sfText_getLocalBounds(text);
		sfText_setPosition(text, (sfVector2f){floor(size.x/2 - rect.width/2), 10});
		sfRenderWindow_drawText(g->g->render, text, NULL);
	}

	return CATCH_NONE;
}

int overlay_cursor(game_t* g)
{
	int cursor = CURSOR_IGNORE;
	int i = overlay_draw(g, 0);
	if (i >= 0)
	{
		char w = i % 3;
		i /= 3;
		if (w == 0) // buttons
		{
			size_t cur = 0;
			char buffer[1024];
			if (0 <= i && i <= 5)
				cur += snprintf(buffer+cur, 1024-cur, "Fenêtre: %s", g->o->sw[i]->name);
			else if (i == 7)
				cur += snprintf(buffer+cur, 1024-cur, "Sauvegarder");
			if (cur != 0)
				draw_tooltip(g->g, g->a, buffer);
		}
		else if (w == 1) // orders
		{
			char buffer[1024];
			size_t n = 1024;
			size_t cur = 0;
			if (i == 0)
			{
			}
			else if (i == 1)
			{
				cur += snprintf(buffer+cur, n-cur, "Rentrer à la maison");
			}
			else if (i <= 9)
			{
				i -= 2;
				kindOf_mine_t* t = &g->u->mines[i];
				cur += snprintf(buffer+cur, n-cur, "Aller à %s", t->name);
			}
			else if (i == 10)
			{
				cur += snprintf(buffer+cur, n-cur, "Attaquer ennemi proche");
			}
			else if (i == 11)
			{
				cur += snprintf(buffer+cur, n-cur, "Attaquer bâtiment proche");
			}
			if (cur != 0)
				draw_tooltip(g->g, g->a, buffer);
		}
		else if (w == 2) // statuses
		{
		}
	}
	else if ((cursor =    ov_build_cursor(&g->o->build,       g)) != CURSOR_IGNORE);
	else if ((cursor =  swbuilding_cursor(&g->o->swbuilding,  g)) != CURSOR_IGNORE);
	else if ((cursor =     switems_cursor(&g->o->switems,     g)) != CURSOR_IGNORE);
	else if ((cursor = swmaterials_cursor(&g->o->swmaterials, g)) != CURSOR_IGNORE);
	else if ((cursor =    swskills_cursor(&g->o->swskills,    g)) != CURSOR_IGNORE);
	else if ((cursor = swequipment_cursor(&g->o->swequipment, g)) != CURSOR_IGNORE);
	else
	{
		sfVector2f pos = world_mouse(g);
		object_t* o = world_objectAt(g->w, pos.x, pos.y, &g->player->o);
		if (o != NULL)
		{
			if (o->t == O_CHARACTER)
			{
				character_t* c = (character_t*) o;
				if (c != g->player)
				{
					if (sfKeyboard_isKeyPressed(sfKeyLControl))
						cursor = CURSOR_ATTACK;
				}

				if (c->ai != NULL)
				{
					draw_tooltip(g->g, g->a, c->ai->name);
				}
			}
			else if (o->t == O_MINE)
			{
				cursor = CURSOR_GATHER;

				mine_t* m = (mine_t*) o;
				transform_t* tr = &m->t->harvest;

				if (!(tr->n_res == 0 || tr->res[0].is_item))
				{
					int id = tr->res[0].id;
					kindOf_material_t* t = &g->u->materials[id];
					int skill = t->skill;

					char buffer[1024];
					float amount = g->player->inventory.materials[id];
					float max = character_maxOfMaterial(g->player, t);
					snprintf(buffer, 1024, "%s (%.0f/%.0f)", g->u->skills[skill].name, floor(amount), floor(max));
					draw_tooltip(g->g, g->a, buffer);
				}
			}
			else if (o->t == O_BUILDING)
			{
				building_t* b = (building_t*) o;
				if (b->owner != g->player->o.uuid)
				{
					if (sfKeyboard_isKeyPressed(sfKeyLControl))
						cursor = CURSOR_ATTACK;
					else if (b->build_progress == 1)
						cursor = CURSOR_ENTER;
				}
				else if (b->build_progress == 1)
					cursor = CURSOR_ENTER;
				else
					cursor = CURSOR_BUILD;

				draw_tooltip(g->g, g->a, b->t->name);
			}
		}
	}

	return cursor == CURSOR_IGNORE ? CURSOR_DEFAULT : cursor;
}

int overlay_catch(game_t* g, int t)
{
	character_t* c = g->player;
	int i = overlay_draw(g, 0);
	if (i != CATCH_NONE)
	{
		int type = CATCH_TYPE(i);
		int idx  = CATCH_IDX (i);
		if (type == CATCH_BUTTON)
		{
			if (0 <= idx && idx <= 5)
			{
				if (t == sfMouseLeft)
					g->o->sw[idx]->visible ^= 1;
			}
			else if (idx == 7)
			{
				game_save(g, "game.save");
			}
		}
		else if (type == CATCH_ORDER)
		{
			if (idx == 0)
			{
			}
			else if (idx == 1)
			{
				building_t* b = building_get(&g->w->objects, c->hasBuilding);
				if (b != NULL)
					character_goto(c, b->o.uuid);
			}
			else if (idx <= 9)
			{
				idx -= 2;
				kindOf_mine_t* t = &g->u->mines[idx];
				mine_t* m = world_findMine(g->w, c->o.x, c->o.y, t);
				if (m != NULL)
					character_goto(c, m->o.uuid);
			}
			else if (idx == 10)
			{
				character_t* t = world_findEnnemyCharacter(g->w, c);
				if (t != NULL)
					character_attack(c, t->o.uuid);
			}
			else if (idx == 11)
			{
				building_t* t = world_findEnnemyBuilding(g->w, c->o.x, c->o.y, c);
				if (t != NULL)
					character_attack(c, t->o.uuid);
			}
		}
		else if (type == CATCH_STATUS)
		{
			if (t == sfMouseLeft)
			{
				character_eatFor(c, idx);
			}
			else if (t == sfMouseRight)
			{
				g->autoEat[idx] ^= 1;
			}
		}
		return 1;
	}

	if (t == -sfMouseMiddle-1)
	{
		for (size_t i = 0; i < g->o->n_subwindows; i++)
			if (subwindow_catch(g->o->sw[i], g->g, t))
			{
				g->o->selected = i;
				break;
			}
		return 1;
	}
	else if (t == sfMouseMiddle)
	{
		if (g->o->selected >= 0)
		{
			g->o->selected = -1;
			return 1;
		}
	}

	return
	 swbuilding_catch(&g->o->swbuilding,  g, t) ||
	    switems_catch(&g->o->switems,     g, t) ||
	swmaterials_catch(&g->o->swmaterials, g, t) ||
	   swskills_catch(&g->o->swskills,    g, t) ||
	swequipment_catch(&g->o->swequipment, g, t) ||
	   ov_build_catch(&g->o->build,       g, t) ||
	0;
}

int overlay_wheel(game_t* g, int d)
{
	for (size_t i = 0; i < g->o->n_subwindows; i++)
		if (subwindow_wheel(g->o->sw[i], g->g, d))
			return 1;
	return 0;
}

void overlay_move(game_t* g)
{
	sfVector2f mouse = overlay_mouse(g);

	float dx = mouse.x - g->o->lastx;
	float dy = mouse.y - g->o->lasty;
	g->o->lastx = mouse.x;
	g->o->lasty = mouse.y;

	if (g->o->selected < 0)
		return;

	subwindow_t* w = g->o->sw[g->o->selected];
	w->x += dx;
	w->y += dy;
}

void overlay_doRound(game_t* g, float duration)
{
	g->o->msgDelay = fmax(0, g->o->msgDelay - duration);
}

void overlay_message(game_t* g, const char* msg, float delay)
{
	free(g->o->msg);
	g->o->msg = strdup(msg);
	g->o->msgDelay = delay;
}

sfVector2f overlay_mouse(game_t* g)
{
	sfVector2i mouse = sfMouse_getPosition((sfWindow*) g->g->render);
	return sfRenderWindow_mapPixelToCoords(g->g->render, mouse, g->g->overlay_view);
}

sfVector2f world_mouse(game_t* g)
{
	sfVector2i mouse = sfMouse_getPosition((sfWindow*) g->g->render);
	return sfRenderWindow_mapPixelToCoords(g->g->render, mouse, g->g->world_view);
}
