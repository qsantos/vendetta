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

#include "swmaterials.h"

#include <stdio.h>
#include <math.h>

#include "../widgets.h"

void swmaterials_init(swmaterials_t* w, graphics_t* g)
{
	subwindow_init(&w->w, g, "Matériaux", 1024-SW_WIDTH*2, SW_HEIGHT);
}

void swmaterials_exit(swmaterials_t* w)
{
	subwindow_exit(&w->w);
}

size_t swmaterials_materialTooltip(char* buffer, size_t n, universe_t* u, kindOf_material_t* m)
{
	size_t cur = 0;

	cur += snprintf(buffer+cur, n-cur, "%s", m->name);

	if (!m->edible)
		return cur;

	cur += snprintf(buffer+cur, n-cur, "\nComestible");
	for (int i = 0; i < N_STATUSES; i++)
	{
		float b = m->eatBonus[i];
		if (b == 0)
			continue;

		char* name = u->statuses[i].name;
		cur += snprintf(buffer+cur, n-cur, "\n%+.1f %s", b, name);
	}

	return cur;
}

int swmaterials_draw(swmaterials_t* w, game_t* g, char do_draw)
{
	if (do_draw)
	{
		if (!subwindow_draw(&w->w, g->g))
			return -1;
	}

	sfVector2f mouse;
	if (!do_draw)
		mouse = subwindow_mouse(&w->w, g->g);

	static sfText* text = NULL;
	if (text == NULL)
	{
		sfColor color = {255, 255, 255, 255};

		text = sfText_create();
		sfText_setFont         (text, g->g->font);
		sfText_setCharacterSize(text, 15);
		sfText_setColor        (text, color);
	}

	float x = 0;
	float y = 0;
	char caught = 0;

	for (size_t i = 0; i < g->u->n_materials; i++)
	{
		float amount = g->player->inventory.materials[i];

		if (amount < 1)
			continue;

		kindOf_material_t* m = &g->u->materials[i];

		// icon
		sfSprite* sprite = g->g->sprites[m->icon_sprite];
		sfIntRect rect = {32*m->icon_index, 0, 32, 32};
		sfSprite_setTextureRect(sprite, rect);
		sfSprite_setPosition(sprite, (sfVector2f){x,y});
		if (do_draw)
			sfRenderWindow_drawSprite(g->g->render, sprite, NULL);
		else
			caught |= sfSprite_contains(sprite, mouse);

		// text
		float max = character_maxOfMaterial(g->player, m);
		char buffer[1024];
		snprintf(buffer, 1024, "%s: %.0f/%.0f", m->name, floor(amount), floor(max));
		sfText_setPosition(text, (sfVector2f){x+32, y+6});
		sfText_setUTF8(text, buffer);
		if (do_draw)
			sfRenderWindow_drawText(g->g->render, text, NULL);
		else
			caught |= sfText_contains(text, mouse);

		y += 32;

		if (caught)
			return i;
	}

	if (do_draw)
		sfRenderWindow_setView(g->g->render, g->g->overlay_view);

	w->w.height = y;
	return -1;
}

int swmaterials_cursor(swmaterials_t* w, game_t* g)
{
	if (!subwindow_cursor(&w->w, g->g))
		return -1;

	int i = swmaterials_draw(w, g, 0);
	if (i < 0)
		return 0;

	char buffer[1024];
	swmaterials_materialTooltip(buffer, 1024, g->u, &g->u->materials[i]);
	draw_tooltip(g->g, buffer);

	if (g->u->materials[i].edible)
		return 11;

	return 0;
}

char swmaterials_catch(swmaterials_t* w, game_t* g, int t)
{
	if (!subwindow_cursor(&w->w, g->g))
		return 0;

	if (t != sfMouseLeft)
		return subwindow_catch(&w->w, g->g, t);

	int i = swmaterials_draw(w, g, 0);
	if (i < 0)
		return subwindow_catch(&w->w, g->g, t);

	character_eat(g->player, i);
	return 1;
}
