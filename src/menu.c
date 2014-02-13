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

#include "menu.h"

#include <stdlib.h>
#include <string.h>

#include "graphics.h"
#include "game.h"

static char draw_button(float x, float y, const char* txt, graphics_t* gr, char do_draw)
{
	sfVector2i imouse = sfMouse_getPosition((sfWindow*) gr->render);
	sfVector2f mouse = {imouse.x, imouse.y};

	static sfSprite* normal = NULL;
	static sfSprite* mini   = NULL;
	if (normal == NULL)
	{
		normal = graphics_sprite(gr, "data/menubutton.png");
		mini   = graphics_sprite(gr, "data/mini.png");
	}
	sfSprite* sprite = strlen(txt) > 1 ? normal : mini;

	sfFloatRect rect = sfSprite_getGlobalBounds(sprite);
	sfVector2f pos = {x-rect.width/2, y-rect.height/2};
	sfSprite_setPosition(sprite, pos);

	char is_in = sfSprite_contains(sprite, mouse);
	sfColor color = is_in ? sfWhite : (sfColor){255,255,255,127};
	sfSprite_setColor(sprite, color);

	if (do_draw)
		sfRenderWindow_drawSprite(gr->render, sprite, NULL);
	else
		return is_in;

	static sfText* text = NULL;
	if (text == NULL)
	{
		text = sfText_create();
		sfText_setFont(text, gr->font);
		sfText_setCharacterSize(text, 25);
	}

	sfText_setUTF8(text, txt);
	rect = sfText_getLocalBounds(text);
	pos = (sfVector2f){x-rect.width/2-rect.left, y-rect.height/2-rect.top};
	sfText_setPosition(text, pos);
	sfRenderWindow_drawText(gr->render, text, NULL);

	return 0;
}
static char mainmenu(graphics_t* gr, settings_t* s, char do_draw)
{
	(void) s;

	sfVector2u size = sfRenderWindow_getSize(gr->render);
	float x = size.x / 2;
	float y = size.y / 2 - 100;
	const char* labels[] =
	{
		"Jouer !",
		"Configurer",
		"Charger",
		"Quitter",
	};

	for (size_t i = 0; i < sizeof(labels)/sizeof(labels[0]); i++)
	{
		if (draw_button(x, y, labels[i], gr, do_draw))
			return i;

		y += 50;
	}

	return -1;
}
static void draw_slider(graphics_t* gr, float x, float y, const char* name, int* v, int min, int max, char do_draw)
{
	sfVector2i imouse = sfMouse_getPosition((sfWindow*) gr->render);
	sfVector2f mouse = {imouse.x, imouse.y};

	static sfText* text = NULL;
	if (text == NULL)
	{
		text = sfText_create();
		sfText_setFont(text, gr->font);
		sfText_setCharacterSize(text, 20);
	}

	{
		sfFloatRect rect = {0,0, 200, 40};
		rect.left = x - rect.width / 2;
		rect.top  = y - rect.height / 2;
		if (sfFloatRect_contains(&rect, mouse.x, mouse.y) && sfMouse_isButtonPressed(sfMouseLeft))
		{
			float r = (mouse.x - rect.left) / rect.width;
			*v = min + r * (max-min);
		}
		if (do_draw)
		{
			float r = (float)(*v - min) / (max-min);
			graphics_drawProgressBar(gr, rect.left, rect.top, rect.width, rect.height, r, -2);
		}
	}

	{
		char buffer[1024];
		snprintf(buffer, 1024, "%s: %i", name, *v);
		sfText_setUTF8(text, buffer);
		sfFloatRect rect = sfText_getLocalBounds(text);
		sfVector2f pos = {x-rect.width/2-rect.left, y-rect.height/2-rect.top};
		sfText_setPosition(text, pos);
		if (do_draw)
			sfRenderWindow_drawText(gr->render, text, NULL);
	}
}
static char configmenu(graphics_t* gr, settings_t* s, char do_draw)
{
	sfVector2u size = sfRenderWindow_getSize(gr->render);
	float x = size.x / 2;
	float y = size.y / 2 - 100;

	sfText* text = NULL;
	if (text == NULL)
	{
		text = sfText_create();
		sfText_setFont(text, gr->font);
		sfText_setCharacterSize(text, 20);
	}
	char buffer[1024];
	snprintf(buffer, 1024, "Seed: %#x", s->seed);
	sfText_setUTF8(text, buffer);
	sfFloatRect rect = sfText_getLocalBounds(text);
	sfVector2f pos = {x-rect.width/2-rect.left, y-rect.height/2-rect.top};
	sfText_setPosition(text, pos);
	sfRenderWindow_drawText(gr->render, text, NULL);

	y += 50;

	draw_slider(gr, x, y, "Largeur", &s->map_width,  100, 2000, do_draw);
	y += 50;
	draw_slider(gr, x, y, "Height",  &s->map_height, 100, 2000, do_draw);
	y += 50;

	if (draw_button(x, y, "Revenir", gr, do_draw))
		return 0;

	return -1;
}
static void play(settings_t* s, graphics_t* gr, char load)
{
	game_t game;
	game_init(&game, s, gr, load);
	game_loop(&game);
	game_exit(&game);
}
void menu(settings_t* s)
{
	graphics_t graphics;
	graphics_t* gr = &graphics;
	graphics_init(gr);

	sfSprite* illustration = graphics_sprite(gr, "data/menu.png");

	sfRenderWindow* render = gr->render;
	char inconfig = 0;
	char stayhere = 1;
	while (stayhere && sfRenderWindow_isOpen(render))
	{
		sfEvent event;
		while (stayhere && sfRenderWindow_pollEvent(render, &event))
		{
			if (event.type == sfEvtClosed)
			{
				stayhere = 0;
			}
			else if (event.type == sfEvtResized)
			{
				sfFloatRect rect = {0,0,event.size.width,event.size.height};
				sfView* view = sfView_createFromRect(rect);
				sfRenderWindow_setView(render, view);
				sfView_destroy(view);
			}
			else if (event.type == sfEvtKeyReleased)
			{
				if (event.key.code == sfKeyEscape)
					stayhere = 0;
				else if (event.key.code == sfKeyReturn)
					play(s, gr, 0);
			}
			else if (event.type == sfEvtMouseButtonReleased)
			{
				if (inconfig)
				{
					if (configmenu(gr, s, 0) == 0)
						inconfig = 0;
				}
				else
				{
					int i = mainmenu(gr, s, 0);
					if (i == 0)
						play(s, gr, 0);
					else if (i == 1)
						inconfig = 1;
					else if (i == 2)
						play(s, gr, 1);
					else if (i == 3)
						stayhere = 0;
				}
			}
		}

		sfRenderWindow_clear(render, sfBlack);

		sfVector2u size  = sfRenderWindow_getSize(render);
		sfFloatRect rect = sfSprite_getGlobalBounds(illustration);
		sfVector2f pos = {size.x/2 - rect.width/2, size.y/2-rect.height/2};
		sfSprite_setPosition(illustration, pos);
		sfRenderWindow_drawSprite(render, illustration, NULL);

		if (inconfig)
			configmenu(gr, s, 1);
		else
			mainmenu(gr, s, 1);

		graphics_drawCursor(gr, 0);
		sfRenderWindow_display(render);
	}

	graphics_exit(gr);
}
