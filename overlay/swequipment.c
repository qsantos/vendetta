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

#include "swequipment.h"

void swequipment_init(swequipment_t* w, graphics_t* g)
{
	subwindow_init(&w->w, g, L"Equipment", 1024-SW_WIDTH*3, SW_HEIGHT);
}

void swequipment_exit(swequipment_t* w)
{
	subwindow_exit(&w->w);
}

char swequipment_cursor(swequipment_t* w, game_t* g, int x, int y)
{
	(void) w;
	(void) g;
	(void) x;
	(void) y;
	return 0;
}

void swequipment_draw(swequipment_t* w, game_t* g)
{
	if (!subwindow_draw(&w->w, g->g))
		return;

	static sfText* text = NULL;
	if (text == NULL)
	{
		sfColor color = {255, 255, 255, 255};

		text = sfText_create();
		sfText_setFont         (text, g->g->font);
		sfText_setCharacterSize(text, 15);
		sfText_setColor        (text, color);
	}

	sfVector2f pos = {0, 0};

	for (int i = 0; i < g->u->n_slots; i++)
	{
		sfText_setPosition(text, pos);
		sfText_setUnicodeString(text, (sfUint32*) g->u->slots[i].name);
		sfRenderWindow_drawText(g->g->render, text, NULL);

		int id = g->player->equipment[i];
		sfUint32* txt = (sfUint32*) (id >= 0 ? g->u->items[id].name : L"-");
		pos.x += 150;
		sfText_setPosition(text, pos);
		sfText_setUnicodeString(text, txt);
		sfRenderWindow_drawText(g->g->render, text, NULL);
		pos.x -= 150;

		pos.y += 20;
	}

	sfRenderWindow_setView(g->g->render, g->g->overlay_view);
}

char swequipment_catch(swequipment_t* w, game_t* g, int _x, int _y, int t)
{
	if (!subwindow_cursor(&w->w, _x, _y))
		return 0;

	if (t != sfMouseLeft)
		return 0;

	sfVector2f cursor = sfRenderWindow_mapPixelToCoords(g->g->render, (sfVector2i){_x,_y}, w->w.view);
	float x = cursor.x;
	float y = cursor.y;

	static sfText* text = NULL;
	if (text == NULL)
	{
		text = sfText_create();
		sfText_setFont         (text, g->g->font);
		sfText_setCharacterSize(text, 18);
	}

	sfVector2f pos = {0, 0};

	pos.y -= 20;
	for (int i = 0; i < g->u->n_slots; i++)
	{
		pos.y += 20;

		int id = g->player->equipment[i];
		sfUint32* txt = (sfUint32*) (id >= 0 ? g->u->items[id].name : L"-");
		pos.x += 150;
		sfText_setPosition(text, pos);
		sfText_setUnicodeString(text, txt);
		pos.x -= 150;

		sfFloatRect rect = sfText_getGlobalBounds(text);
		if (!sfFloatRect_contains(&rect, x, y))
			continue;

		int j = g->player->equipment[i];
		if (j >= 0)
		{
			g->player->equipment[i] = -1;
			g->player->inventory.items[j]++;
		}
		return 1;
	}

	return subwindow_catch(&w->w, _x, _y, t);
}
